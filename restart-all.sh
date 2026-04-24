#!/bin/bash
set -e

ANSIBLE_HOST_KEY_CHECKING=False ansible-playbook \
    -i inventory/nodes.yaml \
    ansible/restart-all.yaml \
    -u andy -K --ask-pass
