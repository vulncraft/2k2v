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

	Scenario: Replay works from existing empty wal
		Given an existing empty wal
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

	Scenario: Replay fails from invalid wal
		Given an existing invalid wal
		When KVNode is started 
		Then the node is not running
		And the wal has not changed
