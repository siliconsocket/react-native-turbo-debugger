#if DEBUG

#import <React/RCTBridgeModule.h>
#import "RNTurboStateCapture.h"

#if RCT_NEW_ARCH_ENABLED
#import "generated/RNTurboDebuggerStateSpec.h"
@interface RNTurboDebuggerState : NSObject <NativeRNTurboDebuggerStateSpec>
#else
@interface RNTurboDebuggerState : NSObject <RCTBridgeModule>
#endif
@end

@implementation RNTurboDebuggerState

RCT_EXPORT_MODULE(RNTurboDebuggerState)

#if RCT_NEW_ARCH_ENABLED
- (void)enqueueState:(NSString *)json {
  RNTDEnqueueState(json);
}

- (std::shared_ptr<facebook::react::TurboModule>)getTurboModule:
    (const facebook::react::ObjCTurboModule::InitParams &)params {
  return std::make_shared<facebook::react::NativeRNTurboDebuggerStateSpecJSI>(params);
}
#else
RCT_EXPORT_METHOD(enqueueState:(NSString *)json) {
  RNTDEnqueueState(json);
}
#endif

@end
#endif
