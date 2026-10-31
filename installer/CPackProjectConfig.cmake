# Loaded by cpack, not by CMake. A self-contained DEB must not install its
# bundled Qt libraries into /usr/lib, where they clash with distro packages.
if(CPACK_GENERATOR STREQUAL "DEB")
    set(CPACK_PACKAGING_INSTALL_PREFIX "/opt/newton")
endif()
