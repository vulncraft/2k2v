import os
import time
import docker
import requests

TEST_IMAGE = os.getenv("TEST_IMAGE", "kvnode:latest")

def before_scenario(context, scenario):
    client = docker.from_env()
    context.container = client.containers.run(
            image=TEST_IMAGE,
            detach=True,
            ports={"3000":3000},
            remove=True,
            command=["-a", "0.0.0.0" ]
            )
    _wait_for_ready(context, "http://0.0.0.0:3000/keys/status")

def after_scenario(context, scenario):
    context.container.stop()

def _wait_for_ready(context, url, timeout=30):
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
