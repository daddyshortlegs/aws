#!/bin/bash
set -e

ANSIBLE_HOST_KEY_CHECKING=False ansible-playbook \
    -i inventory/nodes.yaml \
    backend/restart-all-launchers.yaml \
    -u andy -K --ask-pass
