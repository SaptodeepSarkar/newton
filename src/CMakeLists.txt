qt_add_library(newton_core STATIC
    orbstate.cpp
    orbstate.h
)
target_link_libraries(newton_core PUBLIC Qt6::Core)
target_include_directories(newton_core PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})

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
