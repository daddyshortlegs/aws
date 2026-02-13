package vm

import (
	"context"
	"fmt"

	"github.com/hashicorp/terraform-plugin-framework/path"
	"github.com/hashicorp/terraform-plugin-framework/resource"
	"github.com/hashicorp/terraform-plugin-framework/resource/schema"
	"github.com/hashicorp/terraform-plugin-framework/resource/schema/int64planmodifier"
	"github.com/hashicorp/terraform-plugin-framework/resource/schema/planmodifier"
	"github.com/hashicorp/terraform-plugin-framework/resource/schema/stringdefault"
	"github.com/hashicorp/terraform-plugin-framework/resource/schema/stringplanmodifier"
	"github.com/hashicorp/terraform-plugin-framework/types"
	"github.com/terraform-provider-vm-launcher/local/internal/client"
)

var _ resource.Resource = (*vmResource)(nil)
var _ resource.ResourceWithConfigure = (*vmResource)(nil)

type vmResource struct {
	client *client.Client
}

type vmResourceModel struct {
	ID           types.String `tfsdk:"id"`
	Name         types.String `tfsdk:"name"`
	InstanceType types.String `tfsdk:"instance_type"`
	Region       types.String `tfsdk:"region"`
	SSHPort      types.Int64   `tfsdk:"ssh_port"`
	PID          types.Int64   `tfsdk:"pid"`
}

func NewVMResource() resource.Resource {
	return &vmResource{}
}

func (r *vmResource) Metadata(_ context.Context, req resource.MetadataRequest, resp *resource.MetadataResponse) {
	// Provider type is "vmlauncher", so resource type becomes "vmlauncher_vm".
	// (Terraform splits on first underscore: "vm_launcher_vm" would be provider "vm", resource "launcher_vm".)
	resp.TypeName = req.ProviderTypeName + "_vm"
}

func (r *vmResource) Schema(_ context.Context, _ resource.SchemaRequest, resp *resource.SchemaResponse) {
	resp.Schema = schema.Schema{
		Description: "A VM managed by the VM launcher proxy API.",
		Attributes: map[string]schema.Attribute{
			"id": schema.StringAttribute{
				Description: "Instance ID (UUID) of the VM.",
				Computed:    true,
				PlanModifiers: []planmodifier.String{
					stringplanmodifier.UseStateForUnknown(),
				},
			},
			"name": schema.StringAttribute{
				Description: "Name of the VM.",
				Required:    true,
				PlanModifiers: []planmodifier.String{
					stringplanmodifier.RequiresReplace(),
				},
			},
			"instance_type": schema.StringAttribute{
				Description: "Instance type (e.g. t2.micro).",
				Optional:    true,
				Computed:    true,
				Default:     stringdefault.StaticString("t2.micro"),
			},
			"region": schema.StringAttribute{
				Description: "Region (e.g. us-east-1).",
				Optional:    true,
				Computed:    true,
				Default:     stringdefault.StaticString("us-east-1"),
			},
			"ssh_port": schema.Int64Attribute{
				Description: "SSH port for the VM.",
				Computed:    true,
				PlanModifiers: []planmodifier.Int64{
					int64planmodifier.UseStateForUnknown(),
				},
			},
			"pid": schema.Int64Attribute{
				Description: "Process ID of the VM process on the host.",
				Computed:    true,
				PlanModifiers: []planmodifier.Int64{
					int64planmodifier.UseStateForUnknown(),
				},
			},
		},
	}
}

func (r *vmResource) Configure(_ context.Context, req resource.ConfigureRequest, resp *resource.ConfigureResponse) {
	if req.ProviderData == nil {
		return
	}
	cli, ok := req.ProviderData.(*client.Client)
	if !ok {
		resp.Diagnostics.AddError(
			"Unexpected Provider Data Type",
			fmt.Sprintf("Expected *client.Client, got %T", req.ProviderData),
		)
		return
	}
	r.client = cli
}

func (r *vmResource) Create(ctx context.Context, req resource.CreateRequest, resp *resource.CreateResponse) {
	var plan vmResourceModel
	diags := req.Plan.Get(ctx, &plan)
	resp.Diagnostics.Append(diags...)
	if resp.Diagnostics.HasError() {
		return
	}

	name := plan.Name.ValueString()
	instanceType := plan.InstanceType.ValueString()
	if instanceType == "" {
		instanceType = "t2.micro"
	}
	region := plan.Region.ValueString()
	if region == "" {
		region = "us-east-1"
	}

	out, err := r.client.LaunchVM(name, instanceType, region)
	if err != nil {
		resp.Diagnostics.AddError("Launch VM failed", err.Error())
		return
	}

	plan.ID = types.StringValue(out.InstanceID)
	plan.SSHPort = types.Int64Value(int64(out.SSHPort))
	plan.PID = types.Int64Value(int64(out.PID))
	plan.InstanceType = types.StringValue(instanceType)
	plan.Region = types.StringValue(region)

	diags = resp.State.Set(ctx, plan)
	resp.Diagnostics.Append(diags...)
}

func (r *vmResource) Read(ctx context.Context, req resource.ReadRequest, resp *resource.ReadResponse) {
	var state vmResourceModel
	diags := req.State.Get(ctx, &state)
	resp.Diagnostics.Append(diags...)
	if resp.Diagnostics.HasError() {
		return
	}

	vms, err := r.client.ListVMs()
	if err != nil {
		resp.Diagnostics.AddError("List VMs failed", err.Error())
		return
	}

	id := state.ID.ValueString()
	for _, v := range vms {
		if v.ID == id {
			state.ID = types.StringValue(v.ID)
			state.Name = types.StringValue(v.Name)
			state.SSHPort = types.Int64Value(int64(v.SSHPort))
			state.PID = types.Int64Value(int64(v.PID))
			diags = resp.State.Set(ctx, state)
			resp.Diagnostics.Append(diags...)
			return
		}
	}

	// VM not found - remove from state so Terraform will recreate if needed
	resp.State.RemoveResource(ctx)
}

func (r *vmResource) Update(ctx context.Context, req resource.UpdateRequest, resp *resource.UpdateResponse) {
	// No update support in API; only name/instance_type/region at create time.
	// Just copy state from plan for any computed/optional changes.
	var plan vmResourceModel
	diags := req.Plan.Get(ctx, &plan)
	resp.Diagnostics.Append(diags...)
	if resp.Diagnostics.HasError() {
		return
	}
	diags = resp.State.Set(ctx, plan)
	resp.Diagnostics.Append(diags...)
}

func (r *vmResource) Delete(ctx context.Context, req resource.DeleteRequest, resp *resource.DeleteResponse) {
	var state vmResourceModel
	diags := req.State.Get(ctx, &state)
	resp.Diagnostics.Append(diags...)
	if resp.Diagnostics.HasError() {
		return
	}

	id := state.ID.ValueString()
	if err := r.client.DeleteVM(id); err != nil {
		resp.Diagnostics.AddError("Delete VM failed", err.Error())
		return
	}
}

func (r *vmResource) ImportState(ctx context.Context, req resource.ImportStateRequest, resp *resource.ImportStateResponse) {
	resource.ImportStatePassthroughID(ctx, path.Root("id"), req, resp)
}
