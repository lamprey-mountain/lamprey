import os
import re

def process_file(filepath):
    with open(filepath, 'r') as f:
        content = f.read()
    
    original = content
    
    # Fix duplicate class attributes by merging them
    def merge_class_attributes(match):
        tag = match.group(0)
        # Find all class attributes (including multiline)
        class_matches = re.findall(r'class="([^"]*)"', tag)
        if len(class_matches) > 1:
            # Merge all class values
            merged_classes = ' '.join(class_matches)
            # Remove all class attributes
            tag = re.sub(r'\s*class="[^"]*"', '', tag)
            # Add merged class attribute before the closing >
            tag = tag.rstrip('>')
            tag = tag + f' class="{merged_classes}">'
        return tag
    
    # Match opening button tags (handles multiline)
    pattern = r'<button[^>]*>'
    content = re.sub(pattern, merge_class_attributes, content, flags=re.DOTALL)
    
    if content != original:
        with open(filepath, 'w') as f:
            f.write(content)
        return True
    return False

# Process specific files with errors
files_to_fix = ['./Forum2.tsx', './Wiki.tsx']
count = 0
for filepath in files_to_fix:
    if os.path.exists(filepath):
        if process_file(filepath):
            count += 1
            print(f'Modified: {filepath}')

print(f'Total files modified: {count}')
