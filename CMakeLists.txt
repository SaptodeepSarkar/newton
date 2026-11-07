cmake_minimum_required(VERSION 3.24)
project(Newton VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# 6.9 is the floor, not a preference: earlier Qt links the AGL framework that
# Xcode 26 removed, so macOS builds fail at link time (QTBUG-137687).
find_package(Qt6 6.9 REQUIRED COMPONENTS Core Gui Network Qml Quick Test)
qt_standard_project_setup(REQUIRES 6.9)

if(MSVC)
    add_compile_options(/W4 /permissive-)
else()
    add_compile_options(-Wall -Wextra -Wpedantic)
endif()

enable_testing()
add_subdirectory(src)
add_subdirectory(tests)

include(GNUInstallDirs)

install(TARGETS newton
    BUNDLE DESTINATION .
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
)

# Pulls the Qt runtime and QML imports next to the binary, so the archive runs
# on a machine without Qt installed. The QML lives in the executable's
# resources; the fallback copies it only if this Qt has no deploy helper.
if(COMMAND qt_generate_deploy_qml_app_script)
    qt_generate_deploy_qml_app_script(
        TARGET newton
        OUTPUT_SCRIPT newton_deploy_script
        NO_UNSUPPORTED_PLATFORM_ERROR
    )
    install(SCRIPT ${newton_deploy_script})

    # The deploy helper copies every resolved dependency, glibc and libstdc++
    # included. Shipping those shadows the target's patched system libraries and
    # breaks on other distros, so they are dropped: the host's copies are the
    # correct ones to link against.
    if(UNIX AND NOT APPLE)
        install(CODE "
            foreach(_lib libc.so.6 libm.so.6 libdl.so.2 libpthread.so.0 librt.so.1
                         libgcc_s.so.1 libstdc++.so.6)
                file(GLOB _staged \"\${CMAKE_INSTALL_PREFIX}/lib/\${_lib}*\"
                                   \"\${CMAKE_INSTALL_PREFIX}/lib64/\${_lib}*\")
                file(REMOVE \${_staged})
            endforeach()
        ")
    endif()
else()
    install(DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}/src/
        DESTINATION ${CMAKE_INSTALL_DATADIR}/newton
        FILES_MATCHING PATTERN "*.qml"
    )
endif()

set(CPACK_PACKAGE_NAME "newton")
set(CPACK_PACKAGE_VENDOR "Saptodeep Sarkar")
set(CPACK_PACKAGE_CONTACT "Saptodeep Sarkar <SaptodeepSarkar@users.noreply.github.com>")
set(CPACK_PACKAGE_DESCRIPTION_SUMMARY "Newton, a local voice assistant")
set(CPACK_RESOURCE_FILE_LICENSE "${CMAKE_CURRENT_SOURCE_DIR}/LICENSE")
set(CPACK_PACKAGE_FILE_NAME
    "newton-${PROJECT_VERSION}-${CMAKE_SYSTEM_NAME}-${CMAKE_SYSTEM_PROCESSOR}")
set(CPACK_PROJECT_CONFIG_FILE "${CMAKE_CURRENT_SOURCE_DIR}/installer/CPackProjectConfig.cmake")

if(WIN32)
    set(CPACK_GENERATOR "ZIP")
elseif(APPLE)
    set(CPACK_GENERATOR "DragNDrop")
    set(CPACK_DMG_VOLUME_NAME "Newton")
else()
    set(CPACK_GENERATOR "TGZ;DEB")
    set(CPACK_DEBIAN_PACKAGE_MAINTAINER "${CPACK_PACKAGE_CONTACT}")
endif()

include(CPack)
