
# Scorpio Log Analysis Tool, used for extracting abnormal requests (function not returning)
import re
from collections import defaultdict

def extract_unique_id_lines(input_path, output_path):
    """Extract lines with unique IDs and save them to a file"""
    
    # Compile optimized regular expression (note the comments in re.VERBOSE mode)
    uuid_regex = re.compile(
        r'''^ID:                         # Fixed starting identifier
        \{?                            # Optional left curly brace
        (                              # Start capturing group
          [0-9a-fA-F]{8}               # 8 hexadecimal digits
          -                             # Separator
          (?:[0-9a-fA-F]{4}-){3}       # Three middle groups (non-capturing group for performance)
          [0-9a-fA-F]{12}              # Last 12 digits
        )                              # End capturing group
        \}?                            # Optional right curly brace
        (?!\S)                         # Ensure ID is followed by a space or end of line
        ''', 
        re.IGNORECASE | re.VERBOSE
    )
    uuid_regex = re.compile(
        r'^ID:\{?([0-9a-fA-F]{8}-(?:[0-9a-fA-F]{4}-){3}[0-9a-fA-F]{12})\}?\b',
        re.IGNORECASE
    )
    
    id_counter = defaultdict(int)
    logs = defaultdict(int)
    # First pass: Count occurrences of each ID
    with open(input_path, 'r', encoding='utf-8') as f:
        for line_num, line in enumerate(f, 1):
            # Clean up line content (ignore comments and whitespace)
            line = line.split('#')[0].strip()
            if match := uuid_regex.search(line):
                # Extract the standardized ID (convert to lowercase + remove braces)
                raw_uuid = match.group(1).strip('{}')
                standard_uuid = raw_uuid.lower()
                logs[standard_uuid] = line
                id_counter[standard_uuid] += 1
                
    # Second pass: Record line numbers for unique IDs
    unique_lines = []
    with open(input_path, 'r', encoding='utf-8') as f:
        for line_num, line in enumerate(f, 1):
            line = line.split('#')[0].strip()
            if match := uuid_regex.search(line):
                raw_uuid = match.group(1).strip('{}')
                standard_uuid = raw_uuid.lower()
                if id_counter[standard_uuid] == 1:
                    unique_lines.append(logs[standard_uuid])
                    
    # Write the results to a file (one line number per line)
    with open(output_path, 'w', encoding='utf-8') as f:
        f.write('\n'.join(map(str, unique_lines)))


# For example, run this script with: python log_analysis.py input.log output.txt
if __name__ == "__main__":
    import sys
    if len(sys.argv) != 2:
        print(f"Usage: {sys.argv[0]} input.log")
        sys.exit(1)
    
    extract_unique_id_lines(sys.argv[1], "output.txt")
    print(f"Unique ID line numbers have been saved")