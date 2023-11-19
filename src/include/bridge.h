#ifndef BRIDGE_H
#define BRIDGE_H

#include "rust.h"
using namespace rust;

#ifndef CXXBRIDGE1_STRUCT_ProcessMetadata
#define CXXBRIDGE1_STRUCT_ProcessMetadata
struct ProcessMetadata final {
  String name;
  String shell;
  String command;
  String log_path;
  Vec<String> args;
  using IsRelocatable = std::true_type;
};
#endif

extern "C" int64_t stop(int64_t pid);
extern "C" int64_t run(ProcessMetadata metadata);
#endif
