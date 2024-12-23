prefix = /usr/local

all:
	PATH=/usr/local/bin:/usr/bin:#{PATH} cargo build

install:
	install target/debug/ezkvm $(DESTDIR)$(prefix)/bin

clean:
	rm -rf target
