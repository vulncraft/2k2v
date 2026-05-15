import os
import shutil
import subprocess
import tempfile
import time
import docker
import requests

TEST_IMAGE = os.getenv("TEST_IMAGE", "kvnode:latest")
TEST_BINARY = os.getenv("TEST_BINARY", "ERROR_TEST_BINARY_NOT_SET")

def before_scenario(context, scenario):
    context.pid = None
    context.tmp_dir = tempfile.mkdtemp()

def after_scenario(context, scenario):
    if context.pid is not None:
        context.pid.kill()
    shutil.rmtree(context.tmp_dir)

