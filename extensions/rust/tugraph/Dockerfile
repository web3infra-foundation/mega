FROM c9-ssl32-gh19

# Install dependencies
# libgfortran: TuGraph 4.5.2 (Dockerfile)
RUN dnf update -y \
    && dnf install -y libgfortran

# Install TuGraph 4.5.2
RUN curl -LO https://github.com/TuGraph-family/tugraph-db/releases/download/v4.5.2/tugraph-4.5.2-1.el8.x86_64.rpm \
    && dnf install -y tugraph-4.5.2-1.el8.x86_64.rpm \
    && rm -f tugraph-4.5.2-1.el8.x86_64.rpm
ENV LD_LIBRARY_PATH="/usr/local/lib64/lgraph:$LD_LIBRARY_PATH"

ARG USERNAME="rust"
USER $USERNAME

# Create and set permissions for workdir directory
USER root
RUN mkdir -p /workdir && chown $USERNAME:$USERNAME /workdir
USER $USERNAME

WORKDIR /workdir

# Copy artifacts
COPY ./import_tugraph ./import_tugraph
COPY ./import.config ./import.config
COPY ./.env ./.env

RUN mkdir log
RUN mkdir target

ENV RUST_BACKTRACE=1
CMD ["./import_tugraph"]
