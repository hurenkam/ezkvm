prefix = /usr/local

all:
	PATH=/usr/local/bin:/usr/bin:$(PATH) cargo build

test:
	PATH=/usr/local/bin:/usr/bin:$(PATH) cargo test

install:
	install target/debug/ezkvm $(DESTDIR)$(prefix)/bin

clean:
	rm -rf target
