Pod::Spec.new do |s|
  s.name = 'RNTurboDebugger'
  s.version = '0.1.0'
  s.summary = 'Debug-only native HTTP capture for React Native'
  s.homepage = 'https://github.com/siliconsocket/react-native-turbo-debugger'
  s.license = { :type => 'MIT', :file => '../LICENSE' }
  s.author = 'Silicon Socket'
  s.source = { :git => 'https://github.com/siliconsocket/react-native-turbo-debugger.git', :tag => s.version.to_s }
  s.ios.deployment_target = '15.1'
  s.source_files = 'RNTurboNetworkCapture.m'
  s.requires_arc = true
  s.dependency 'React-RCTNetwork'
end
