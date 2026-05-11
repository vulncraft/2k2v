import json
import requests
from behave import given, when, then

BASE_URL = "http://0.0.0.0:3000"

@given("KVNode is running")
def step_kvnode_running(context):
    response = requests.get(f"{BASE_URL}/key/status")
    assert response.status_code != 500

@given("KVNode has initial state")
def step_apply_initial_state(context):
    headers = {"Content-Type": "application/json"}
    state = json.loads(context.text)
    for each in state:
        k = each["key"]
        v = each["value"]
        assert requests.put(f"{BASE_URL}/key", json={"key": k, "value": v}, headers=headers).status_code == 200


@when("I get key={key}")
def step_get_key(context, key):
    context.response = requests.get(f"{BASE_URL}/key/{key}")

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
