qt_add_library(newton_core STATIC
    audio/AudioCapture.cpp
    audio/AudioCapture.h
    audio/Mic.cpp
    audio/Mic.h
    audio/Speaker.cpp
    audio/Speaker.h
    audio/WakeModel.cpp
    audio/WakeModel.h
    controller.cpp
    controller.h
    daemon.cpp
    orbstate.cpp
    orbstate.h
)
target_link_libraries(newton_core PUBLIC Qt6::Core Qt6::Network)
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

# The QML lives in its own static library so the tests can load the same
# module resources the app ships, not a copy.
qt_add_library(newton_qml STATIC)
qt_add_qml_module(newton_qml
    URI Newton
    VERSION 1.0
    QML_FILES
        Main.qml
        Ring.qml
        Theme.qml
        FirstRun.qml
        Settings.qml
        MicBadge.qml
)
target_link_libraries(newton_qml PUBLIC Qt6::Quick Qt6::Qml)

qt_add_executable(newton main.cpp)
target_link_libraries(newton PRIVATE newton_core newton_qml newton_qmlplugin)

set_target_properties(newton PROPERTIES
    WIN32_EXECUTABLE TRUE
    MACOSX_BUNDLE TRUE
    MACOSX_BUNDLE_BUNDLE_NAME "Newton"
)
