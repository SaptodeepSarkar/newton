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
