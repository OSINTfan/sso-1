
import json
import sys

def strip_pda(obj):
    if isinstance(obj, dict):
        if "pda" in obj:
            del obj["pda"]
        
        # Rename fields for compatibility
        if "writable" in obj:
            obj["isMut"] = obj.pop("writable")
        if "signer" in obj:
            obj["isSigner"] = obj.pop("signer")
            
        if "docs" in obj:
            del obj["docs"]
            
        for k, v in obj.items():
            strip_pda(v)
    elif isinstance(obj, list):
        for item in obj:
            strip_pda(item)

def main():
    if len(sys.argv) < 2:
        print("Usage: python patch_idl.py <idl_path>")
        sys.exit(1)
    
    path = sys.argv[1]
    with open(path, 'r') as f:
        data = json.load(f)
    
    strip_pda(data)
    
    with open(path, 'w') as f:
        json.dump(data, f, indent=2)
    
    print(f"Patched {path}")

if __name__ == "__main__":
    main()
