# These deliberately use `=` and not `:=` so that client makefiles can
# augment HOST_RPATH_DIR / TARGET_RPATH_DIR.
HOST_RPATH_ENV = \
    $(LD_LIB_PATH_ENVVAR)="$(TMPDIR):$(HOST_RPATH_DIR):$($(LD_LIB_PATH_ENVVAR))"
TARGET_RPATH_ENV = \
    $(LD_LIB_PATH_ENVVAR)="$(TMPDIR):$(TARGET_RPATH_DIR):$($(LD_LIB_PATH_ENVVAR))"

RUSTC_ORIGINAL := $(RUSTC)
BARE_RUSTC := $(HOST_RPATH_ENV) '$(RUSTC)'
BARE_RUSTDOC := $(HOST_RPATH_ENV) '$(RUSTDOC)'
RUSTC := $(BARE_RUSTC) --out-dir $(TMPDIR) -L $(TMPDIR) $(RUSTFLAGS)
RUSTDOC := $(BARE_RUSTDOC) -L $(TARGET_RPATH_DIR)
ifdef RUSTC_LINKER
RUSTC := $(RUSTC) -Clinker=$(RUSTC_LINKER)
RUSTDOC := $(RUSTDOC) --linker $(RUSTC_LINKER) -Z unstable-options
endif
#CC := $(CC) -L $(TMPDIR)
HTMLDOCCK := $(PYTHON) $(S)/src/etc/htmldocck.py
CGREP := "$(S)/src/etc/cat-and-grep.sh"

# This is the name of the binary we will generate and run; use this
# e.g. for `$(CC) -o $(RUN_BINFILE)`.
RUN_BINFILE = $(TMPDIR)/$(1)

# RUN and FAIL are basic way we will invoke the generated binary.  On
# non-windows platforms, they set the LD_LIBRARY_PATH environment
# variable before running the binary.

RLIB_GLOB = lib$(1)*.rlib
BIN = $(1)

UNAME = $(shell uname)

ifeq ($(UNAME),Darwin)
RUN = $(TARGET_RPATH_ENV) $(RUN_BINFILE)
FAIL = $(TARGET_RPATH_ENV) $(RUN_BINFILE) && exit 1 || exit 0
DYLIB_GLOB = lib$(1)*.dylib
DYLIB = $(TMPDIR)/lib$(1).dylib
STATICLIB = $(TMPDIR)/lib$(1).a
STATICLIB_GLOB = lib$(1)*.a
else
ifdef IS_WINDOWS
RUN = PATH="$(PATH):$(TARGET_RPATH_DIR)" $(RUN_BINFILE)
FAIL = PATH="$(PATH):$(TARGET_RPATH_DIR)" $(RUN_BINFILE) && exit 1 || exit 0
DYLIB_GLOB = $(1)*.dll
DYLIB = $(TMPDIR)/$(1).dll
STATICLIB = $(TMPDIR)/$(1).lib
STATICLIB_GLOB = $(1)*.lib
BIN = $(1).exe
else
RUN = $(TARGET_RPATH_ENV) $(RUN_BINFILE)
FAIL = $(TARGET_RPATH_ENV) $(RUN_BINFILE) && exit 1 || exit 0
DYLIB_GLOB = lib$(1)*.so
DYLIB = $(TMPDIR)/lib$(1).so
STATICLIB = $(TMPDIR)/lib$(1).a
STATICLIB_GLOB = lib$(1)*.a
endif
endif

ifdef IS_MSVC
COMPILE_OBJ = $(CC) -c -Fo:`cygpath -w $(1)` $(2)
COMPILE_OBJ_CXX = $(CXX) -c -Fo:`cygpath -w $(1)` $(2)
NATIVE_STATICLIB_FILE = $(1).lib
NATIVE_STATICLIB = $(TMPDIR)/$(call NATIVE_STATICLIB_FILE,$(1))
OUT_EXE=-Fe:`cygpath -w $(TMPDIR)/$(call BIN,$(1))` \
	-Fo:`cygpath -w $(TMPDIR)/$(1).obj`
else
COMPILE_OBJ = $(CC) -c -o $(1) $(2)
COMPILE_OBJ_CXX = $(CXX) -c -o $(1) $(2)
NATIVE_STATICLIB_FILE = lib$(1).a
NATIVE_STATICLIB = $(call STATICLIB,$(1))
OUT_EXE=-o $(TMPDIR)/$(1)
endif


# Extra flags needed to compile a working executable with the standard library
ifdef IS_WINDOWS
ifdef IS_MSVC
	EXTRACFLAGS := ws2_32.lib userenv.lib shell32.lib advapi32.lib
else
	EXTRACFLAGS := -lws2_32 -luserenv
endif
else
ifeq ($(UNAME),Darwin)
	EXTRACFLAGS := -lresolv
else
ifeq ($(UNAME),FreeBSD)
	EXTRACFLAGS := -lm -lpthread -lgcc_s
else
ifeq ($(UNAME),Bitrig)
	EXTRACFLAGS := -lm -lpthread
	EXTRACXXFLAGS := -lc++ -lc++abi
else
ifeq ($(UNAME),SunOS)
	EXTRACFLAGS := -lm -lpthread -lposix4 -lsocket -lresolv
else
ifeq ($(UNAME),OpenBSD)
	EXTRACFLAGS := -lm -lpthread -lc++abi
	RUSTC := $(RUSTC) -C linker="$(word 1,$(CC:ccache=))"
else
	EXTRACFLAGS := -lm -lrt -ldl -lpthread
	EXTRACXXFLAGS := -lstdc++
endif
endif
endif
endif
endif
endif

REMOVE_DYLIBS     = rm $(TMPDIR)/$(call DYLIB_GLOB,$(1))
REMOVE_RLIBS      = rm $(TMPDIR)/$(call RLIB_GLOB,$(1))

%.a: %.o
	$(AR) crus $@ $<
ifdef IS_MSVC
%.lib: lib%.o
	$(MSVC_LIB) -out:`cygpath -w $@` $<
else
%.lib: lib%.o
	$(AR) crus $@ $<
endif
%.dylib: %.o
	$(CC) -dynamiclib -Wl,-dylib -o $@ $<
%.so: %.o
	$(CC) -o $@ $< -shared

ifdef IS_MSVC
%.dll: lib%.o
	$(CC) $< -link -dll -out:`cygpath -w $@`
else
%.dll: lib%.o
	$(CC) -o $@ $< -shared
endif

$(TMPDIR)/lib%.o: %.c
	$(call COMPILE_OBJ,$@,$<)
