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

export interface Volume {
  id: string;
  name: string;
  mount_path: string;
  loop_device?: string;
}
