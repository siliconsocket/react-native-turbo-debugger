#if DEBUG

#import <Foundation/Foundation.h>
#import <React/RCTHTTPRequestHandler.h>
#import <TargetConditionals.h>
#import <arpa/inet.h>
#import <errno.h>
#import <objc/runtime.h>
#import <sys/socket.h>
#import <unistd.h>

static char RNTDObservedRequest;

static uint64_t RNTDNow(void) {
  return (uint64_t)(NSProcessInfo.processInfo.systemUptime * 1000.0);
}

// The writer owns the socket and never blocks URLSession's delegate queue.
@interface RNTDWriter : NSObject
+ (void)emit:(NSDictionary *)event;
@end

@implementation RNTDWriter
+ (void)emit:(NSDictionary *)event {
#if TARGET_OS_SIMULATOR
  static int socketFD = -1;
  static uint64_t retryAt = 0;
  static dispatch_queue_t queue;
  static dispatch_semaphore_t slots;
  static dispatch_once_t token;
  dispatch_once(&token, ^{
    queue = dispatch_queue_create("com.siliconsocket.rnturbodebugger.writer", DISPATCH_QUEUE_SERIAL);
    slots = dispatch_semaphore_create(256);
  });
  if (dispatch_semaphore_wait(slots, DISPATCH_TIME_NOW) != 0) return;
  dispatch_async(queue, ^{
    @try {
      if (RNTDNow() < retryAt) return;
      BOOL connected = socketFD >= 0;
      if (!connected) {
        socketFD = [self connectToDebugger];
        connected = socketFD >= 0 && [self writeFrame:[self hello] to:socketFD];
      }
      if (!connected || ![self writeFrame:event to:socketFD]) {
        if (socketFD >= 0) close(socketFD);
        socketFD = -1;
        retryAt = RNTDNow() + 500;
      }
    } @finally {
      dispatch_semaphore_signal(slots);
    }
  });
#endif
}

+ (NSDictionary *)hello {
  NSBundle *bundle = NSBundle.mainBundle;
  NSString *app = [bundle objectForInfoDictionaryKey:@"CFBundleDisplayName"]
      ?: [bundle objectForInfoDictionaryKey:@"CFBundleName"] ?: @"React Native";
  NSString *device = NSProcessInfo.processInfo.environment[@"SIMULATOR_DEVICE_NAME"] ?: @"iOS Simulator";
  return @{@"type": @"hello", @"version": @1, @"app": app, @"device": device, @"platform": @"iOS"};
}

+ (int)connectToDebugger {
  int fd = socket(AF_INET, SOCK_STREAM, 0);
  if (fd < 0) return -1;
  int disableSigpipe = 1;
  struct timeval timeout = {0, 250000};
  setsockopt(fd, SOL_SOCKET, SO_NOSIGPIPE, &disableSigpipe, sizeof(disableSigpipe));
  setsockopt(fd, SOL_SOCKET, SO_SNDTIMEO, &timeout, sizeof(timeout));

  struct sockaddr_in address = {0};
  address.sin_len = sizeof(address);
  address.sin_family = AF_INET;
  address.sin_port = htons(7410);
  address.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
  if (connect(fd, (const struct sockaddr *)&address, sizeof(address)) == 0) return fd;
  close(fd);
  return -1;
}

+ (BOOL)writeFrame:(NSDictionary *)message to:(int)fd {
  NSData *payload = [NSJSONSerialization dataWithJSONObject:message options:0 error:NULL];
  if (payload.length == 0 || payload.length > 16384) return NO;
  uint32_t size = htonl((uint32_t)payload.length);
  NSMutableData *frame = [NSMutableData dataWithBytes:&size length:sizeof(size)];
  [frame appendData:payload];
  const uint8_t *bytes = frame.bytes;
  NSUInteger remaining = frame.length;
  while (remaining > 0) {
    ssize_t sent = send(fd, bytes, remaining, 0);
    if (sent < 0 && errno == EINTR) continue;
    if (sent <= 0) return NO;
    bytes += sent;
    remaining -= (NSUInteger)sent;
  }
  return YES;
}
@end

// Observe the handler that React Native already uses. The original NSURLSession
// still performs the request, including uploads, redirects, cookies and caching.
@interface RNTDRequestObserver : NSObject <RCTURLRequestDelegate>
@property (nonatomic, strong) id<RCTURLRequestDelegate> downstream;
@property (nonatomic, copy) NSString *requestID;
@property (nonatomic) BOOL reported;
- (instancetype)initWithRequest:(NSURLRequest *)request delegate:(id<RCTURLRequestDelegate>)delegate;
- (void)report:(NSString *)outcome status:(NSNumber *)status;
@end

@implementation RNTDRequestObserver
- (instancetype)initWithRequest:(NSURLRequest *)request delegate:(id<RCTURLRequestDelegate>)delegate {
  if ((self = [super init])) {
    _downstream = delegate;
    _requestID = NSUUID.UUID.UUIDString;
    NSURLComponents *url = [NSURLComponents componentsWithURL:request.URL resolvingAgainstBaseURL:YES];
    NSString *path = [(url.host ?: @"") stringByAppendingString:url.percentEncodedPath ?: @"/"];
    if (path.length > 2048) path = [path substringToIndex:2048];
    [RNTDWriter emit:@{
      @"type": @"start", @"id": _requestID, @"atMs": @(RNTDNow()),
      @"method": request.HTTPMethod ?: @"GET", @"path": path
    }];
  }
  return self;
}

- (void)report:(NSString *)outcome status:(NSNumber *)status {
  @synchronized (self) {
    if (self.reported) return;
    self.reported = YES;
    [RNTDWriter emit:@{
      @"type": @"finish", @"id": self.requestID, @"atMs": @(RNTDNow()),
      @"outcome": outcome, @"status": status ?: NSNull.null
    }];
  }
}

- (void)URLRequest:(id)token didSendDataWithProgress:(int64_t)bytesSent {
  [self.downstream URLRequest:token didSendDataWithProgress:bytesSent];
}

- (void)URLRequest:(id)token didReceiveResponse:(NSURLResponse *)response {
  if ([response isKindOfClass:NSHTTPURLResponse.class]) {
    [self report:@"response" status:@(((NSHTTPURLResponse *)response).statusCode)];
  }
  [self.downstream URLRequest:token didReceiveResponse:response];
}

- (void)URLRequest:(id)token didReceiveData:(NSData *)data {
  [self.downstream URLRequest:token didReceiveData:data];
}

- (void)URLRequest:(id)token didCompleteWithError:(NSError *)error {
  BOOL cancelled = [error.domain isEqualToString:NSURLErrorDomain] && error.code == NSURLErrorCancelled;
  if (error != nil) [self report:cancelled ? @"cancelled" : @"failed" status:nil];
  objc_setAssociatedObject(token, &RNTDObservedRequest, nil, OBJC_ASSOCIATION_RETAIN_NONATOMIC);
  [self.downstream URLRequest:token didCompleteWithError:error];
}
@end

@implementation RCTHTTPRequestHandler (RNTurboDebugger)
+ (void)load {
#if TARGET_OS_SIMULATOR
  Method send = class_getInstanceMethod(self, @selector(sendRequest:withDelegate:));
  Method observe = class_getInstanceMethod(self, @selector(rntd_sendRequest:withDelegate:));
  Method cancel = class_getInstanceMethod(self, @selector(cancelRequest:));
  Method observeCancel = class_getInstanceMethod(self, @selector(rntd_cancelRequest:));
  if (send && observe && cancel && observeCancel) {
    method_exchangeImplementations(send, observe);
    method_exchangeImplementations(cancel, observeCancel);
  }
#endif
}

- (id)rntd_sendRequest:(NSURLRequest *)request withDelegate:(id<RCTURLRequestDelegate>)delegate {
  RNTDRequestObserver *observer = [[RNTDRequestObserver alloc] initWithRequest:request delegate:delegate];
  id token = [self rntd_sendRequest:request withDelegate:observer];
  if (token == nil) {
    [observer report:@"failed" status:nil];
  } else {
    objc_setAssociatedObject(token, &RNTDObservedRequest, observer, OBJC_ASSOCIATION_RETAIN_NONATOMIC);
  }
  return token;
}

- (void)rntd_cancelRequest:(id)token {
  RNTDRequestObserver *observer = objc_getAssociatedObject(token, &RNTDObservedRequest);
  [observer report:@"cancelled" status:nil];
  [self rntd_cancelRequest:token];
  objc_setAssociatedObject(token, &RNTDObservedRequest, nil, OBJC_ASSOCIATION_RETAIN_NONATOMIC);
}
@end
#endif
