export interface VM {
  id: string;
  name: string;
  ssh_host: string;
  ssh_port: number;
  pid: number;
}

export interface VMListResponse {
  vms: VM[];
}
