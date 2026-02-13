package provider

import (
	"context"

	"github.com/hashicorp/terraform-plugin-framework/datasource"
	"github.com/hashicorp/terraform-plugin-framework/path"
	"github.com/hashicorp/terraform-plugin-framework/provider"
	"github.com/hashicorp/terraform-plugin-framework/provider/schema"
	"github.com/hashicorp/terraform-plugin-framework/resource"
	"github.com/hashicorp/terraform-plugin-framework/types"
	"github.com/terraform-provider-vm-launcher/local/internal/client"
	"github.com/terraform-provider-vm-launcher/local/internal/resource/vm"
)

const defaultProxyURL = "http://127.0.0.1:8080"

var _ provider.Provider = (*vmLauncherProvider)(nil)

type vmLauncherProvider struct{}

type vmLauncherProviderModel struct {
	ProxyBaseURL types.String `tfsdk:"proxy_base_url"`
}

func New() provider.Provider {
	return &vmLauncherProvider{}
}

func (p *vmLauncherProvider) Metadata(_ context.Context, _ provider.MetadataRequest, resp *provider.MetadataResponse) {
	resp.TypeName = "vmlauncher"
}

func (p *vmLauncherProvider) Schema(_ context.Context, _ provider.SchemaRequest, resp *provider.SchemaResponse) {
	resp.Schema = schema.Schema{
		Attributes: map[string]schema.Attribute{
			"proxy_base_url": schema.StringAttribute{
				Optional:    true,
				Description: "Base URL of the VM launcher proxy (e.g. http://127.0.0.1:8080). Defaults to http://127.0.0.1:8080.",
			},
		},
	}
}

func (p *vmLauncherProvider) Configure(ctx context.Context, req provider.ConfigureRequest, resp *provider.ConfigureResponse) {
	var config vmLauncherProviderModel
	diags := req.Config.Get(ctx, &config)
	resp.Diagnostics.Append(diags...)
	if resp.Diagnostics.HasError() {
		return
	}

	baseURL := defaultProxyURL
	if !config.ProxyBaseURL.IsNull() && !config.ProxyBaseURL.IsUnknown() && config.ProxyBaseURL.ValueString() != "" {
		baseURL = config.ProxyBaseURL.ValueString()
	}

	// Validate format
	if baseURL == "" {
		resp.Diagnostics.AddAttributeError(
			path.Root("proxy_base_url"),
			"Invalid proxy_base_url",
			"proxy_base_url must be a non-empty URL (e.g. http://127.0.0.1:8080)",
		)
		return
	}

	cli := client.NewClient(baseURL)
	resp.ResourceData = cli
}

func (p *vmLauncherProvider) DataSources(_ context.Context) []func() datasource.DataSource {
	return nil
}

func (p *vmLauncherProvider) Resources(_ context.Context) []func() resource.Resource {
	return []func() resource.Resource{
		vm.NewVMResource,
	}
}
