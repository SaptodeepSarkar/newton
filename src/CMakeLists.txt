qt_add_library(newton_core STATIC
    audio/AudioCapture.cpp
    audio/AudioCapture.h
    orbstate.cpp
    orbstate.h
)
target_link_libraries(newton_core PUBLIC Qt6::Core)
target_include_directories(newton_core PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})

# Microphone capture uses Qt Multimedia when the host has it. Without it the
# device path compiles out and the ring buffer stays fully testable.
find_package(Qt6 QUIET COMPONENTS Multimedia)
if(TARGET Qt6::Multimedia)
    target_link_libraries(newton_core PUBLIC Qt6::Multimedia)
    target_compile_definitions(newton_core PUBLIC NEWTON_HAS_MULTIMEDIA)
endif()

# Without this the module treats Theme as a normal type and its functions are
# not callable from bindings.
set_source_files_properties(Theme.qml PROPERTIES QT_QML_SINGLETON_TYPE TRUE)

qt_add_executable(newton main.cpp)
qt_add_qml_module(newton
    URI Newton
    VERSION 1.0
    QML_FILES
        Main.qml
        Ring.qml
        Theme.qml
)
target_link_libraries(newton PRIVATE newton_core Qt6::Quick Qt6::Qml)

set_target_properties(newton PROPERTIES
    WIN32_EXECUTABLE TRUE
    MACOSX_BUNDLE TRUE
    MACOSX_BUNDLE_BUNDLE_NAME "Newton"
)
