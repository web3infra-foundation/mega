FROM localhost:30500/almalinux:9.6-cratespro-20251023

ARG USERNAME="rust"
USER $USERNAME

# Create and set permissions for workdir directory
USER root
RUN mkdir -p /workdir && chown $USERNAME:$USERNAME /workdir
USER $USERNAME

WORKDIR /workdir

# Copy artifacts
COPY ./cve ./cve
COPY ./.env ./.env

RUN mkdir log

ENV RUST_BACKTRACE=1
CMD ["./cve"]
