package client

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"
)

// Client talks to the VM launcher proxy API.
type Client struct {
	BaseURL    string
	HTTPClient *http.Client
}

// LaunchVMRequest is the body for POST /launch-vm.
type LaunchVMRequest struct {
	Name         string `json:"name"`
	InstanceType string `json:"instance_type"`
	Region       string `json:"region"`
}

// LaunchVMResponse is the response from POST /launch-vm.
type LaunchVMResponse struct {
	Success    bool   `json:"success"`
	Message    string `json:"message"`
	InstanceID string `json:"instance_id"`
	SSHPort    int    `json:"ssh_port"`
	PID        int    `json:"pid"`
}

// VMInfo is a single VM from GET /list-vms.
type VMInfo struct {
	ID      string `json:"id"`
	Name    string `json:"name"`
	SSHPort int    `json:"ssh_port"`
	PID     int    `json:"pid"`
}

// DeleteVMRequest is the body for DELETE /delete-vm.
type DeleteVMRequest struct {
	ID string `json:"id"`
}

// NewClient returns a client for the given proxy base URL (e.g. http://127.0.0.1:8080).
func NewClient(baseURL string) *Client {
	baseURL = strings.TrimSuffix(baseURL, "/")
	return &Client{
		BaseURL: baseURL,
		HTTPClient: &http.Client{
			Timeout: 30 * time.Second,
		},
	}
}

// LaunchVM creates a VM via POST /launch-vm.
func (c *Client) LaunchVM(name, instanceType, region string) (*LaunchVMResponse, error) {
	body := LaunchVMRequest{
		Name:         name,
		InstanceType: instanceType,
		Region:       region,
	}
	jsonBody, err := json.Marshal(body)
	if err != nil {
		return nil, err
	}

	req, err := http.NewRequest(http.MethodPost, c.BaseURL+"/launch-vm", bytes.NewReader(jsonBody))
	if err != nil {
		return nil, err
	}
	req.Header.Set("Content-Type", "application/json")

	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, err
	}

	var out LaunchVMResponse
	if err := json.Unmarshal(data, &out); err != nil {
		return nil, fmt.Errorf("decode launch-vm response: %w", err)
	}
	if !out.Success {
		return nil, fmt.Errorf("launch-vm failed: %s", out.Message)
	}
	return &out, nil
}

// ListVMs returns all VMs from GET /list-vms.
// Backend may return a raw array or an object with "vms" key.
func (c *Client) ListVMs() ([]VMInfo, error) {
	req, err := http.NewRequest(http.MethodGet, c.BaseURL+"/list-vms", nil)
	if err != nil {
		return nil, err
	}

	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, err
	}

	// Try array first
	var arr []VMInfo
	if err := json.Unmarshal(data, &arr); err == nil {
		return arr, nil
	}
	// Try { "vms": [...] }
	var wrapper struct {
		VMs []VMInfo `json:"vms"`
	}
	if err := json.Unmarshal(data, &wrapper); err != nil {
		return nil, fmt.Errorf("decode list-vms response: %w", err)
	}
	return wrapper.VMs, nil
}

// DeleteVM deletes a VM via DELETE /delete-vm.
func (c *Client) DeleteVM(id string) error {
	body := DeleteVMRequest{ID: id}
	jsonBody, err := json.Marshal(body)
	if err != nil {
		return err
	}

	req, err := http.NewRequest(http.MethodDelete, c.BaseURL+"/delete-vm", bytes.NewReader(jsonBody))
	if err != nil {
		return err
	}
	req.Header.Set("Content-Type", "application/json")

	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		data, _ := io.ReadAll(resp.Body)
		return fmt.Errorf("delete-vm returned %d: %s", resp.StatusCode, string(data))
	}
	return nil
}
