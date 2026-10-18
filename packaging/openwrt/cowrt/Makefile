include $(TOPDIR)/rules.mk

PKG_NAME:=cowrt
PKG_VERSION:=$(COWRT_VERSION)
PKG_RELEASE:=1
PKG_MAINTAINER:=Lantharos
PKG_LICENSE:=Proprietary

include $(INCLUDE_DIR)/package.mk

define Package/cowrt
  SECTION:=admin
  CATEGORY:=Administration
  TITLE:=Modern web interface for OpenWrt
  URL:=https://github.com/Lantharos/coWRT
  DEPENDS:=+rpcd +rpcd-mod-iwinfo +ip +tc-full +kmod-nf-conntrack-netlink +kmod-sched-cake +kmod-sched-ctinfo +kmod-ifb
endef

define Package/cowrt/description
  A fast, friendly web interface for managing your router: devices, Wi-Fi,
  network status and system health.
endef

define Package/cowrt/conffiles
/etc/config/cowrt
endef

define Build/Prepare
	mkdir -p $(PKG_BUILD_DIR)
endef

define Build/Compile
endef

define Package/cowrt/install
	$(INSTALL_DIR) $(1)/usr/sbin $(1)/usr/share/cowrt $(1)/etc/init.d $(1)/etc/config $(1)/etc/uci-defaults $(1)/lib/upgrade/keep.d
	$(INSTALL_BIN) $(COWRT_STAGE)/cowrtd $(1)/usr/sbin/cowrtd
	$(CP) $(COWRT_STAGE)/www $(1)/usr/share/cowrt/
	$(INSTALL_BIN) ./files/cowrtd.init $(1)/etc/init.d/cowrtd
	$(INSTALL_CONF) ./files/cowrt.config $(1)/etc/config/cowrt
	$(INSTALL_BIN) ./files/cowrt.defaults $(1)/etc/uci-defaults/90-cowrt
	$(INSTALL_DATA) ./files/cowrt.keep $(1)/lib/upgrade/keep.d/cowrt
endef

define Package/cowrt/postinst
#!/bin/sh
[ -z "$${IPKG_INSTROOT}" ] && [ "$${PKG_UPGRADE}" = "1" ] && /etc/init.d/cowrtd restart
exit 0
endef

define Package/cowrt/prerm
#!/bin/sh
[ -n "$${IPKG_INSTROOT}" ] && exit 0
uci -q get uhttpd.main.listen_http | grep -qE ':8080( |$$)' || exit 0
uci -q batch <<-EOF
	delete uhttpd.main.listen_http
	add_list uhttpd.main.listen_http='0.0.0.0:80'
	add_list uhttpd.main.listen_http='[::]:80'
	delete uhttpd.main.listen_https
	add_list uhttpd.main.listen_https='0.0.0.0:443'
	add_list uhttpd.main.listen_https='[::]:443'
	commit uhttpd
EOF
/etc/init.d/uhttpd running && /etc/init.d/uhttpd restart
exit 0
endef

$(eval $(call BuildPackage,cowrt))
