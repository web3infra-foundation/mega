#include "memory_info.h"
#include <mach/mach.h>
#include <mach/mach_host.h>
#include <stdio.h>

int get_memory_info(MemoryInfo *info) {
    mach_port_t host_port;
    mach_msg_type_number_t host_size;
    vm_size_t pagesize;
    vm_statistics64_data_t vm_stat;

    host_port = mach_host_self();
    host_size = sizeof(vm_stat) / sizeof(integer_t);

    host_page_size(host_port, &pagesize);

    if (host_statistics64(host_port, HOST_VM_INFO, (host_info64_t)&vm_stat, &host_size) != KERN_SUCCESS) {
        return 1;
    }
    info->free_memory = (uint64_t)vm_stat.free_count * (uint64_t)pagesize;
    info->active_memory = (uint64_t)vm_stat.active_count * (uint64_t)pagesize;
    info->inactive_memory = (uint64_t)vm_stat.inactive_count * (uint64_t)pagesize;
    info->wired_memory = (uint64_t)vm_stat.wire_count * (uint64_t)pagesize;
    info->total_memory = (info->free_memory + info->active_memory + info->inactive_memory + info->wired_memory);

    return 0;
}