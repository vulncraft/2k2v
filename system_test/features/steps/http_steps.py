import json
import requests
from behave import given, when, then

from features.steps.common import BASE_URL




@when("I get key={key}")
def step_get_key(context, key):
    context.response = requests.get(f"{BASE_URL}/key/{key}")

@when("I get ready")
def step_get_ready(context):
    context.response = requests.get(f"{BASE_URL}/ready")

@when("I get health")
def step_get_health(context):
    context.response = requests.get(f"{BASE_URL}/health")

@when("I put {key}={value}")
def step_get_key(context, key, value):
    
    headers = {"Content-Type": "application/json"}
    context.response = requests.put(f"{BASE_URL}/key", json={"key": key, "value": value}, headers=headers)

@when("I delete key={key}")
def step_get_key(context, key):
    
    headers = {"Content-Type": "application/json"}
    context.response = requests.delete(f"{BASE_URL}/key/{key}")

@then("the response status is {code:d}")
def step_status_check(context, code):
    assert context.response.status_code == code, f"Expected {code} got {context.response.status_code}"

@then("the response body is")
def step_body_check(context):
    expected = json.loads(context.text)
    actual = context.response.json()
    assert expected == actual, (
            f"expected: \n{expected}",
            f"actual: \n{actual}"
            )
