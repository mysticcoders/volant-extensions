# Generates MicroPython's embed package (py core sources and headers) for this configuration, plus the
# JSON module from extmod. Run by ../build.sh with MICROPYTHON_TOP, PACKAGE_DIR, BUILD and CC set.
SRC_QSTR += $(MICROPYTHON_TOP)/extmod/modjson.c
include $(MICROPYTHON_TOP)/ports/embed/embed.mk
