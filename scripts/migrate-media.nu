# migrate to the new s3 media path system
for path in (s5cmd ls --show-fullpath s3://chat-files/media/* | lines) {
  s5cmd mv $path ($path)/file
}

for path in (s5cmd ls --show-fullpath s3://chat-files/thumb/*/original | lines) {
  let id = ($path | parse --regex '/thumb/(?<id>[a-f0-9-]+)').0.id
  s5cmd mv $path $"s3://chat-files/media/($id)/poster"
}

s5cmd rm s3://chat-files/emoji/*
s5cmd rm s3://chat-files/thumb/*
