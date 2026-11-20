# Include after ReactNative-application.cmake, from a Debug app build.
add_subdirectory("${RNTD_ANDROID_DIR}/build/generated/source/codegen/jni"
                 "${CMAKE_CURRENT_BINARY_DIR}/rntd-state-codegen")
target_link_libraries(react_codegen_RNTurboDebuggerStateSpec common_flags)
target_link_libraries(${CMAKE_PROJECT_NAME} react_codegen_RNTurboDebuggerStateSpec)
target_compile_definitions(${CMAKE_PROJECT_NAME} PRIVATE RNTD_STATE_ENABLED)
