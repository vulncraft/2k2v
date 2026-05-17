
import json
import pathlib
import shlex
from behave import When, given, when, then
import os
import subprocess
import time

import requests


TEST_BINARY = pathlib.Path(os.getenv("TEST_BINARY", "ERROR_TEST_BINARY_NOT_SET")).absolute()
BASE_URL = "http://0.0.0.0:3000"
READY_URL = "http://0.0.0.0:3000/ready"

@given("KVNode is running")
def spawn_clean_node(context):
    spawn_node(context)
    _wait_for_ready(context, READY_URL)

@when("KVNode is started")
def start_kvnode_no_check(context):
    spawn_node(context)

@then("the node is not running")
def check_pid_status_not_running(context):
    time.sleep(2)
    assert context.pid.poll() is not None

@then("the wal has not changed")
def check_wal_not_changed(context):
    with open(context.wal_path, "rb") as f:

        data = f.read()
        assert data == context.wal_init_value, f"received: {data}  expected: {context.wal_init_value}"



@given("KVNode has initial state")
def step_apply_initial_state(context):
    spawn_clean_node(context)
    headers = {"Content-Type": "application/json"}
    state = json.loads(context.text)
    for each in state:
        k = each["key"]
        v = each["value"]
        assert requests.put(f"{BASE_URL}/key", json={"key": k, "value": v}, headers=headers).status_code == 200

@When("I restart the node")
def restart_node(context):
    clean_shutdown(context)
    try:
        requests.get(f"{BASE_URL}/key/status")
    except:
        assert True
    else:
        assert False
    spawn_clean_node(context)

@given("an existing empty wal")
def init_empty_wal(context):
    wal_path = f"{context.tmp_dir}/wal.bin"
    open(wal_path, "a").close()
    context.wal_path = wal_path
    context.wal_init_value = b''

@given("an existing invalid wal")
def init_invalid_wal(context):
    wal_path = f"{context.tmp_dir}/wal.bin"
    
    with open(wal_path, "ab") as f:
        f.write(b'A' * 1024)
    context.wal_init_value = b'A' * 1024
    context.wal_path = wal_path

def spawn_node(context):
    context.pid = subprocess.Popen(shlex.split(f"{TEST_BINARY} --file wal.bin --address 0.0.0.0"), cwd=context.tmp_dir)

def _wait_for_ready(context, url, timeout=10):
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            r = requests.get(url, timeout=2)
            if r.status_code < 500:
                return
        except requests.ConnectionError:
            pass
        time.sleep(0.5)
    raise RuntimeError(f"App did not become ready within {timeout}s")

def clean_shutdown(context):
    context.pid.kill()
