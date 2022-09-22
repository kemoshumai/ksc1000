FROM rust:1.63-buster

ARG DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y wget git 

RUN apt-get install -y gpg lsb-release software-properties-common

RUN wget https://apt.llvm.org/llvm.sh
RUN chmod +x llvm.sh
RUN ./llvm.sh 10

RUN update-alternatives --install /usr/bin/clang clang /usr/bin/clang-10 1
RUN update-alternatives --install /usr/bin/clang++ clang++ /usr/bin/clang++-10 1
RUN update-alternatives --install /usr/bin/llvm-config llvm-config /usr/bin/llvm-config-10 10
RUN update-alternatives --config llvm-config

ENV PATH $PATH:/usr/lib/llvm-10/bin