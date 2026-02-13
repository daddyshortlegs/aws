package main

import (
	"context"
	"log"

	"github.com/hashicorp/terraform-plugin-framework/providerserver"
	"github.com/terraform-provider-vm-launcher/local/internal/provider"
)

func main() {
	err := providerserver.Serve(context.Background(), provider.New, providerserver.ServeOpts{
		Address: "localhost/myorg/vm-launcher",
	})
	if err != nil {
		log.Fatal(err)
	}
}
