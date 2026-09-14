import os
import sys

# This function checks whether the configuration file exists and is readable.
def load_config(path):
    if not os.path.exists(path):
        # The file does not exist, so we return an empty dictionary.
        return {}

    with open(path) as f:
        data = f.read()

    result = {}
    for line in data.splitlines():
        if line.startswith("#") or line.strip() == "":
            continue
        key, value = line.split("=", 1)
        result[key.strip()] = value.strip()
    return result  # the parsed configuration
