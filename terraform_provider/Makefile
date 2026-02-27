.PHONY: build audit clean

BINARY := terraform-provider-aws2

audit:
	govulncheck ./...

build: audit
	go build -o $(BINARY) .

clean:
	rm -f $(BINARY)
