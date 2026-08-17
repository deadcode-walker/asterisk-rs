FROM andrius/asterisk:22@sha256:7a22d773fe0f81adb715cd3e8df57c602726f8ef9d39deead6360e051483e280 AS mailbox-builder

USER root
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential curl libedit-dev libjansson-dev libsqlite3-dev libssl-dev \
        libxml2-dev pkg-config uuid-dev \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /usr/src/asterisk
RUN curl -fsSL -o /tmp/asterisk.tar.gz \
        https://downloads.asterisk.org/pub/telephony/asterisk/releases/asterisk-22.10.1.tar.gz \
    && echo "0953564c44fa49827f3c9d70ca6e80db83828c9848440852c6be44c961855353  /tmp/asterisk.tar.gz" \
        | sha256sum -c - \
    && tar --strip-components=1 -xzf /tmp/asterisk.tar.gz \
    && rm /tmp/asterisk.tar.gz \
    && ./configure \
    && make menuselect/menuselect menuselect-tree menuselect.makeopts \
    && menuselect/menuselect --enable res_mwi_external menuselect.makeopts \
    && menuselect/menuselect --enable res_stasis_mailbox menuselect.makeopts \
    && menuselect/menuselect --enable res_ari_mailboxes menuselect.makeopts \
    && make -j4 res

FROM andrius/asterisk:22@sha256:7a22d773fe0f81adb715cd3e8df57c602726f8ef9d39deead6360e051483e280
COPY --from=mailbox-builder /usr/src/asterisk/res/res_mwi_external.so /usr/lib/asterisk/modules/
COPY --from=mailbox-builder /usr/src/asterisk/res/res_stasis_mailbox.so /usr/lib/asterisk/modules/
COPY --from=mailbox-builder /usr/src/asterisk/res/res_ari_mailboxes.so /usr/lib/asterisk/modules/
COPY conf/ /etc/asterisk/
