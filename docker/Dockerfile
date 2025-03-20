ARG UBUNTU_VERSION
FROM ubuntu:${UBUNTU_VERSION}

ARG DEBIAN_FRONTEND=noninteractive
RUN apt update && apt install ca-certificates -y
RUN sed -i 's|http://archive.ubuntu.com/ubuntu/|https://mirror.nju.edu.cn/ubuntu/|g' /etc/apt/sources.list.d/ubuntu.sources
RUN apt update \
    && apt upgrade -y \
    && apt install --no-install-recommends --yes \
    bison \
    build-essential \
    clang \
    clang-format \
    clang-tools \
    gcc-multilib \
    g++-multilib \
    cmake \
    curl \
    doxygen \
    expect \
    flex \
    git \
    libboost-all-dev \
    libcap-dev \
    libffi-dev \
    libgoogle-perftools-dev \
    libncurses-dev \
    libsqlite3-dev \
    libssl-dev \
    libtcmalloc-minimal4t64 \
    lib32stdc++-13-dev \
    libgmp-dev \
    libgmpxx4ldbl \
    lld \
    llvm \
    ncurses-doc \
    ninja-build \
    perl \
    pkg-config \
    python3 \
    python3-colored \
    python3-minimal \
    python3-pip \
    python3-setuptools \
    python3-tabulate \
    python3-termcolor \
    python3-toml \
    subversion \
    sudo \
    unzip \
    wget \
    zsh \
    zsh-syntax-highlighting \
    zsh-autosuggestions \
    # Cleanup
    && apt autoremove \
    && apt clean \
    # Install Python packages
    && pip3 install --no-cache-dir --break-system-packages \
    argparse \
    lit \
    wllvm

ARG USERNAME
ARG USER_UID
ARG USER_GID
ARG USER_HOME=/home/${USERNAME}
ARG FUZZ_TARGET_GENERATOR_DIR=${USER_HOME}/RuMono

ENV PATH="${USER_HOME}/.cargo/bin:${PATH}"
ENV PATH="${FUZZ_TARGET_GENERATOR_DIR}/build/x86_64-unknown-linux-gnu/stage2/bin:${PATH}"
ENV PATH="${FUZZ_TARGET_GENERATOR_DIR}/build/x86_64-unknown-linux-gnu/stage2-tools-bin:${PATH}"

RUN (groupadd --gid=${USER_GID} ${USERNAME} || true) \
    && (useradd --shell=/usr/bin/zsh --uid=${USER_UID} --gid=${USER_GID} --create-home ${USERNAME} || true) \
    && echo "${USERNAME}  ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers

#install rustup
USER ${USERNAME}
WORKDIR ${USER_HOME}

RUN sh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)"

RUN curl --location https://sh.rustup.rs > /tmp/rustup \
    && sh /tmp/rustup -y --default-toolchain=none \
    && rustup --version \
    && rm /tmp/rustup

#create a .zshrc
RUN echo "export PATH=\"${PATH}\"" >> ${USER_HOME}/.zshrc \
    && echo "ulimit -c 0" >> ${USER_HOME}/.zshrc

RUN sed -i 's/^ZSH_THEME=.*/ZSH_THEME="ys"/' ~/.zshrc

RUN echo 'source /usr/share/zsh-autosuggestions/zsh-autosuggestions.zsh\n\
source /usr/share/zsh-syntax-highlighting/zsh-syntax-highlighting.zsh\n' >> ${USER_HOME}/.zshrc

# change default cargo config
SHELL ["/usr/bin/zsh", "-c"]
RUN echo $'[source.crates-io]\n\
replace-with = \'tuna\'\n\
[source.tuna]\n\
registry = "sparse+https://mirrors.tuna.tsinghua.edu.cn/crates.io-index/"\n\
[registries.tuna]\n\
index = "sparse+https://mirrors.tuna.tsinghua.edu.cn/crates.io-index/"\n' > ${USER_HOME}/.cargo/config.toml

ENV USER="${USERNAME}"

RUN mkdir ${FUZZ_TARGET_GENERATOR_DIR}
WORKDIR ${FUZZ_TARGET_GENERATOR_DIR}
ENTRYPOINT [ "/usr/bin/zsh" ]
