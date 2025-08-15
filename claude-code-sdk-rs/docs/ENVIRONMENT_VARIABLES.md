# Environment Variables

## CLAUDE_CODE_MAX_OUTPUT_TOKENS

Controls the maximum number of output tokens that Claude CLI will generate in a single response.

### Valid Range
- **Minimum**: 1
- **Maximum Safe Value**: 32000
- **Recommended Default**: 8192

### Important Notes

1. **Maximum Limit**: The maximum safe value is **32000**. Values above this may cause:
   - Claude CLI to exit with error code 1
   - Timeouts or hanging processes
   - Unpredictable behavior

2. **SDK Protection**: As of v0.1.9, the Rust SDK automatically handles invalid values:
   - Values > 32000 are automatically capped at 32000
   - Non-numeric values are replaced with 8192
   - This ensures your application won't crash due to invalid settings

3. **Setting the Variable**:
   ```bash
   # Maximum safe value
   export CLAUDE_CODE_MAX_OUTPUT_TOKENS=32000
   
   # Conservative recommended value
   export CLAUDE_CODE_MAX_OUTPUT_TOKENS=8192
   
   # Remove the variable (use Claude's default)
   unset CLAUDE_CODE_MAX_OUTPUT_TOKENS
   ```

4. **Common Issues**:
   - `Error: Invalid env var CLAUDE_CODE_MAX_OUTPUT_TOKENS: 50000` - Value too high
   - Process exits immediately with code 1 - Invalid value
   - Timeouts during generation - Value may be too high

### Testing Your Configuration

You can test if your value works with:

```bash
CLAUDE_CODE_MAX_OUTPUT_TOKENS=32000 echo "Say hello" | claude --dangerously-skip-permissions
```

If the command hangs or errors, reduce the value.

## Other Environment Variables

### CLAUDE_CODE_ENTRYPOINT
- Set automatically by the SDK to identify the source of the request
- Default: `"sdk-rust"`

### Standard Variables
The SDK respects standard environment variables like:
- `PATH` - To find the Claude CLI binary
- `HOME` - For locating configuration files
- Standard Node.js/npm environment variables