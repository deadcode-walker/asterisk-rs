FROM andrius/asterisk:22@sha256:7a22d773fe0f81adb715cd3e8df57c602726f8ef9d39deead6360e051483e280 AS mailbox-builder

USER root
RUN sed -i \
        -e 's|http://deb.debian.org/debian-security|https://snapshot.debian.org/archive/debian-security/20260803T000000Z|' \
        -e 's|http://deb.debian.org/debian|https://snapshot.debian.org/archive/debian/20260803T000000Z|' \
        /etc/apt/sources.list.d/debian.sources \
    && echo 'Acquire::Check-Valid-Until "false";' > /etc/apt/apt.conf.d/99snapshot \
    && apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential=12.12 curl=8.14.1-2+deb13u4 libedit-dev=3.1-20250104-1 \
        libjansson-dev=2.14-2+b3 libsqlite3-dev=3.46.1-7+deb13u1 \
        libssl-dev=3.5.6-1~deb13u2 \
        libxml2-dev=2.12.7+dfsg+really2.9.14-2.1+deb13u3 pkg-config=1.8.1-4 \
        uuid-dev=2.41-5 \
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
    && menuselect/menuselect --disable-category MENUSELECT_RES menuselect.makeopts \
    && menuselect/menuselect --enable res_mwi_external menuselect.makeopts \
    && menuselect/menuselect --enable res_stasis_mailbox menuselect.makeopts \
    && menuselect/menuselect --enable res_ari_mailboxes menuselect.makeopts \
    && make -j4 res

FROM andrius/asterisk:22@sha256:7a22d773fe0f81adb715cd3e8df57c602726f8ef9d39deead6360e051483e280
COPY --from=mailbox-builder /usr/src/asterisk/res/res_mwi_external.so /usr/lib/asterisk/modules/
COPY --from=mailbox-builder /usr/src/asterisk/res/res_stasis_mailbox.so /usr/lib/asterisk/modules/
COPY --from=mailbox-builder /usr/src/asterisk/res/res_ari_mailboxes.so /usr/lib/asterisk/modules/
COPY conf/ /etc/asterisk/
