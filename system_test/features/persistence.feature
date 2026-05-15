Feature: KVNode persistent storage

	Scenario: Replay works
		Given KVNode has initial state
		"""
		[
		{"key":"fish", "value":"fishval"},
		{"key":"fish1", "value":"fishval1"},
		{"key":"fish2", "value":"fishval2"}
		]
		"""
		When I get key=fish1
		Then the response status is 200
		When I restart the node
		And I get key=fish1
		Then the response status is 200
