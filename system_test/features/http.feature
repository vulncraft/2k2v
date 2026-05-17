Feature: KVNode HTTP API
	Scenario: health endpoints
		Given KVNode is running
		When I get health
		Then the response status is 200
		When I get ready
		Then the response status is 200

	Scenario: Get a nonexistent key
		Given KVNode is running
		When I get key=unknown
		Then the response status is 404
		And the response body is
		"""
		{"key":"unknown", "value": null}
		"""

	Scenario: Add and get a key
		Given KVNode is running
		When I get key=fish
		Then the response status is 404
		When I put fish=fishval
		Then the response status is 200 
		When I get key=fish
		Then the response status is 200
		And the response body is
		"""
		{"key":"fish", "value":"fishval"}
		"""

	Scenario: Add and delete a key
		Given KVNode is running
		When I put fish=fishval
		When I delete key=fish
		When I get key=fish
		Then the response status is 404
		And the response body is
		"""
		{"key":"fish", "value":null}
		"""

	Scenario: Modify a key
		Given KVNode has initial state
		"""
		[
		{"key":"fish", "value":"fishval"},
		{"key":"fish1", "value":"fishval1"},
		{"key":"fish2", "value":"fishval2"}
		]
		"""
		When I put fish1=modified
		Then the response status is 200
		When I get key=fish1
		Then the response status is 200
		And the response body is
		"""
		{"key":"fish1", "value":"modified"}
		"""
