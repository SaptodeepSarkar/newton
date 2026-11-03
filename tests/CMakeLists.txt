qt_add_executable(tst_orbstate tst_orbstate.cpp)

target_link_libraries(tst_orbstate PRIVATE newton_core Qt6::Test)

add_test(NAME orbstate COMMAND tst_orbstate)

qt_add_executable(tst_audiocapture tst_audiocapture.cpp)
target_link_libraries(tst_audiocapture PRIVATE newton_core Qt6::Test)
add_test(NAME audiocapture COMMAND tst_audiocapture)

# Loads the real QML and walks all five states. Catches a broken binding in a
# state nobody opened yet, without a display.
add_test(NAME qml_smoke COMMAND newton --check)
set_tests_properties(qml_smoke PROPERTIES
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software"
    TIMEOUT 30
)

qt_add_executable(tst_theme tst_theme.cpp)
target_link_libraries(tst_theme PRIVATE newton_core Qt6::Test Qt6::Qml)
add_test(NAME theme COMMAND tst_theme)

qt_add_executable(tst_screens tst_screens.cpp)
target_link_libraries(tst_screens PRIVATE newton_core newton_qml newton_qmlplugin Qt6::Gui Qt6::Test Qt6::Quick)
add_test(NAME screens COMMAND tst_screens)
# QGuiApplication is required by Text; the platform stays offscreen and no
# window is ever created.
set_tests_properties(screens PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen")
