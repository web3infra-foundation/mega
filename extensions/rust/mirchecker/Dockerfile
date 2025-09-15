FROM c9-ssl32-gh19

# Install tools and dependencies
# MirChecker (Dockerfile): gmp-devel, mpfr-devel, ppl-devel, z3-devel
RUN dnf update -y \
    && dnf group install -y "Development Tools" \
    && dnf install -y glibc-langpack-en sudo tzdata \
    && dnf install -y gmp-devel mpfr-devel \
    && dnf install -y epel-release \
    && dnf install -y ppl-devel z3-devel

# Switch to user (user 'rust' with UID 1000 already created in base image)
ARG USERNAME="rust"
ARG USER_UID="1000"
USER $USERNAME

# Install Rust with nightly-2025-01-10 toolchain and required components
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain nightly-2025-01-10 -y
ENV PATH="/home/$USERNAME/.cargo/bin:$PATH"
# Install required components for MirChecker
RUN rustup component add clippy rustfmt rustc-dev rust-src rust-std llvm-tools-preview --toolchain nightly-2025-01-10

# Create and set permissions for workdir directory
USER root
RUN mkdir -p /workdir && chown $USERNAME:$USERNAME /workdir
USER $USERNAME

WORKDIR /workdir

# Copy artifacts for analysis-mirchecker
COPY  ./analysis_mirchecker ./analysis_mirchecker
COPY ./.env ./.env

# Copy artifacts for MirChecker
COPY ./cargo-mir-checker ./cargo-mir-checker
COPY ./mir-checker ./mir-checker
ENV LD_LIBRARY_PATH="/home/$USERNAME/.rustup/toolchains/nightly-2025-01-10-x86_64-unknown-linux-gnu/lib:/usr/local/lib64:/usr/lib64"

ENV TZ=Asia/Shanghai
ENV LANG=en_US.UTF-8
ENV LC_ALL=en_US.UTF-8

ENV RUST_BACKTRACE=1
CMD ["./analysis_mirchecker"]
