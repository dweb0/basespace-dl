# Cookbook

Only download the top 5 biggest files from a project.

```bash
basespace-dl $PROJECT -FF | sort -k1,1hr | head -n 5 | awk '{print $2}' | basespace-dl $PROJECT -f -
```

Only download the Undetermined files for a project

```bash
basespace-dl $PROJECT -U -p "Undetermined"
```
