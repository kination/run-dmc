# run_dmc

__Until now, this project is for research, not for production__

This project __run_dmc__(=> 'Runtime' for 'Data Management Container') is another "Open Container Initiative (OCI) Runtime" written in Rust, specifically optimized for **Data Engineering** workloads.

## Goal of research
- **Performance First**: Engineered for ultra-low latency (<50ms startup) and minimal memory footprint.
- **I/O Optimized**: Leverages **`io_uring`** with polling mode for high-throughput ETL and database operations.
- **Hardware Accelerated**: Native support for **GPU passthrough** and HugePages configuration for AI/ML pipelines.
- **Modern Linux**: Built on **cgroups v2** and modern kernel namespaces.

## Status
**🚧 Under Development (Phase 1)**

Currently implementing the foundational OCI specification compliance (CLI parsing, Basic Isolation).

