Pod::Spec.new do |s|
  s.name = 'RNTurboDebugger'
  s.version = '0.2.0'
  s.summary = 'Debug-only native HTTP and state capture for React Native'
  s.homepage = 'https://github.com/siliconsocket/react-native-turbo-debugger'
  s.license = { :type => 'MIT', :file => '../LICENSE' }
  s.author = 'Silicon Socket'
  s.source = { :git => 'https://github.com/siliconsocket/react-native-turbo-debugger.git', :tag => s.version.to_s }
  s.ios.deployment_target = '15.1'
  s.source_files = '*.{m,mm,h}'
  if ENV['RCT_NEW_ARCH_ENABLED'] == '1'
    Pod::Executable.execute_command('node', [File.join(__dir__, 'generate-state-spec.js')])
    s.source_files = ['*.{m,mm,h}', 'generated/*.{h,mm}']
    s.private_header_files = 'generated/*.h'
    install_modules_dependencies(s)
  end
  s.requires_arc = true
  s.dependency 'React-RCTNetwork'
end
