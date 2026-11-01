FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    gcc \
    libc6-dev \
    libcurl4 \
    libegl1 \
    libgl1 \
    libgl1-mesa-dri \
    libglib2.0-0 \
    libgtk-3-0 \
    libgtk-3-dev \
    libjpeg-turbo8 \
    libnotify4 \
    libpango-1.0-0 \
    libpangocairo-1.0-0 \
    libpng16-16 \
    libsecret-1-0 \
    libsm6 \
    libstdc++6 \
    libunwind8 \
    libusb-1.0-0 \
    libwebkit2gtk-4.0-37 \
    libx11-6 \
    libxext6 \
    libxrender1 \
    libxtst6 \
    libxxf86vm1 \
    pkg-config \
    xvfb \
    && rm -rf /var/lib/apt/lists/*

RUN apt-get update && apt-get install -y --no-install-recommends \
    rustc \
    && rm -rf /var/lib/apt/lists/*

COPY gtk_automation_hook.rs /usr/local/src/gtk_automation_hook.rs

RUN rustc --edition=2021 --crate-type=cdylib -C opt-level=2 \
    /usr/local/src/gtk_automation_hook.rs \
    -o /usr/local/lib/libciq_gtk_hook.so
