#ifndef MEMORY_INFO_H
#define MEMORY_INFO_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct MemoryInfo {
    uint64_t free_memory;
    uint64_t active_memory;
    uint64_t inactive_memory;
    uint64_t wired_memory;
    uint64_t total_memory;
} MemoryInfo;

int get_memory_info(MemoryInfo *info);

#ifdef __cplusplus
}
#endif

#endif
