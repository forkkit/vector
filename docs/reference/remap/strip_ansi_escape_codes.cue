package metadata

remap: functions: strip_ansi_escape_codes: {
	arguments: [
		{
			name:        "value"
			description: "The text to strip."
			required:    true
			type:        "string"
		},
	]
	return:   "string"
	category: "text"
	description: #"""
			Removes the any ansi escape codes from the provided string.
		"""#
	examples: [
		{
			title: "Success"
			input: {
				text: #"\x1b[46mfoo\x1b[0m bar"#
			}
			source: #"""
				.text = strip_ansi_escape_codes(.text)
				"""#
			output: {
				text: "foo bar"
			}
		},
		{
			title: "Error"
			input: {
				text: 37
			}
			source: #"""
				.text = strip_ansi_escape_codes(.text)
			"""#
			output: {
				error: remap.errors.ArgumentError
			}
		},
	]
}
