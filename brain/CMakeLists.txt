cmake_minimum_required(VERSION 3.24)

project(makise-brain LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

find_package(Protobuf REQUIRED)
find_package(gRPC CONFIG REQUIRED)
find_program(GRPC_CPP_PLUGIN_EXECUTABLE grpc_cpp_plugin REQUIRED)

set(MAKISE_PROTO_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../proto")
set(MAKISE_WORLD_PROTO "${MAKISE_PROTO_ROOT}/makise/v1/world.proto")
set(MAKISE_GENERATED_ROOT "${CMAKE_CURRENT_BINARY_DIR}/generated")
set(MAKISE_GENERATED_DIR "${MAKISE_GENERATED_ROOT}/makise/v1")
set(MAKISE_PROTO_SRCS
    "${MAKISE_GENERATED_DIR}/world.pb.cc"
    "${MAKISE_GENERATED_DIR}/world.grpc.pb.cc"
)
set(MAKISE_PROTO_HDRS
    "${MAKISE_GENERATED_DIR}/world.pb.h"
    "${MAKISE_GENERATED_DIR}/world.grpc.pb.h"
)

add_custom_command(
    OUTPUT ${MAKISE_PROTO_SRCS} ${MAKISE_PROTO_HDRS}
    COMMAND ${CMAKE_COMMAND} -E make_directory "${MAKISE_GENERATED_DIR}"
    COMMAND ${Protobuf_PROTOC_EXECUTABLE}
        --proto_path=${MAKISE_PROTO_ROOT}
        --proto_path=${Protobuf_INCLUDE_DIRS}
        --cpp_out=${MAKISE_GENERATED_ROOT}
        --grpc_out=${MAKISE_GENERATED_ROOT}
        --plugin=protoc-gen-grpc=${GRPC_CPP_PLUGIN_EXECUTABLE}
        "${MAKISE_WORLD_PROTO}"
    DEPENDS "${MAKISE_WORLD_PROTO}"
    VERBATIM
)

add_library(makise-world-proto STATIC ${MAKISE_PROTO_SRCS} ${MAKISE_PROTO_HDRS})
target_include_directories(makise-world-proto PUBLIC "${MAKISE_GENERATED_ROOT}")
target_link_libraries(makise-world-proto PUBLIC protobuf::libprotobuf gRPC::grpc++)

add_library(makise-world-client STATIC
    src/world_client.cpp
    src/world_client.hpp
)
target_include_directories(makise-world-client PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/src")
target_link_libraries(makise-world-client PUBLIC makise-world-proto)

if(CMAKE_CXX_COMPILER_ID MATCHES "Clang|GNU")
    target_compile_options(makise-world-proto PRIVATE -Wall -Wextra -Wpedantic)
    target_compile_options(makise-world-client PRIVATE -Wall -Wextra -Wpedantic -Werror)
endif()

include(CTest)
if(BUILD_TESTING)
    add_executable(world-client-compile-test tests/world_client_compile_test.cpp)
    target_link_libraries(world-client-compile-test PRIVATE makise-world-client)

    add_executable(world-client-integration-test tests/world_client_integration_test.cpp)
    target_link_libraries(world-client-integration-test PRIVATE makise-world-client)

    if(CMAKE_CXX_COMPILER_ID MATCHES "Clang|GNU")
        target_compile_options(world-client-compile-test PRIVATE -Wall -Wextra -Wpedantic -Werror)
        target_compile_options(world-client-integration-test PRIVATE -Wall -Wextra -Wpedantic -Werror)
    endif()

    add_test(NAME world-client-compile-test COMMAND world-client-compile-test)

    find_program(BASH_EXECUTABLE bash)
    find_program(CARGO_EXECUTABLE cargo)
    if(BASH_EXECUTABLE AND CARGO_EXECUTABLE)
        add_test(
            NAME world-client-integration-test
            COMMAND "${BASH_EXECUTABLE}" "${CMAKE_CURRENT_SOURCE_DIR}/tests/run_world_client_integration.sh"
                "$<TARGET_FILE:world-client-integration-test>" "${CARGO_EXECUTABLE}" "${CMAKE_CURRENT_SOURCE_DIR}/.."
        )
        set_tests_properties(world-client-integration-test PROPERTIES TIMEOUT 120)
    endif()
endif()
