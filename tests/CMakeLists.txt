qt_add_executable(tst_orbstate tst_orbstate.cpp)

target_link_libraries(tst_orbstate PRIVATE newton_core Qt6::Test)

add_test(NAME orbstate COMMAND tst_orbstate)

# Loads the real QML and walks all five states. Catches a broken binding in a
# state nobody opened yet, without a display.
add_test(NAME qml_smoke COMMAND newton --check)
set_tests_properties(qml_smoke PROPERTIES
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software"
    TIMEOUT 30
)
