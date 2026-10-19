#----------------------------------------------------------------
# Generated CMake target import file for configuration "Release".
#----------------------------------------------------------------

# Commands may need to know the format version.
set(CMAKE_IMPORT_FILE_VERSION 1)

# Import target "DaedalusSimSdk::DaedalusSimSdk" for configuration "Release"
set_property(TARGET DaedalusSimSdk::DaedalusSimSdk APPEND PROPERTY IMPORTED_CONFIGURATIONS RELEASE)
set_target_properties(DaedalusSimSdk::DaedalusSimSdk PROPERTIES
  IMPORTED_LINK_INTERFACE_LANGUAGES_RELEASE "CXX"
  IMPORTED_LOCATION_RELEASE "${_IMPORT_PREFIX}/lib/libdaedalus_sim_sdk.a"
  )

list(APPEND _cmake_import_check_targets DaedalusSimSdk::DaedalusSimSdk )
list(APPEND _cmake_import_check_files_for_DaedalusSimSdk::DaedalusSimSdk "${_IMPORT_PREFIX}/lib/libdaedalus_sim_sdk.a" )

# Commands beyond this point should not need to know the version.
set(CMAKE_IMPORT_FILE_VERSION)
