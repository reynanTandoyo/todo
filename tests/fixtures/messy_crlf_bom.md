<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 1 -->
# Messy Kitchen Tasks   

A preamble table:

| Item | Qty |
|---|---|
| Flour | 2 kg |

```markdown
## Fake Section Inside Backtick Fence
### T-999 · This is not a real task
- status: done
<!-- todo-format: 99 -->
<<<<<<< fake conflict inside fence
```   

````markdown
```
### T-995 · Still inside 4-backtick fence
```
````

   ~~~
### T-998 · Fake task inside indented tilde fence
- status: blocked
~~~

  <!-- multi-line HTML comment
## Fake Section Inside Comment
### T-997 · Fake task inside comment
- note (2026-11-15, ghost): ignored
still ignored -->

## 10. Prep Work   
<!-- todo-format: 2 -->

### T-010 · Bake sourdough bread   
- status: open  
- owner: —
- priority: P1 · type: baking · area: oven
- custom-field_1: hydration 75%   
- note (not-a-date): kept as prose, not a structured note

| Step | Minutes |
|---|---|
| Autolyse | 45 |

#### Sub-heading inside task
Some notes with `### T-996 · inline` text.

### T11 Missing hyphen is a fake task
### T-12NoSpace is a fake task
### T-13 ·   
### ADR-001 · Ordinary heading with different prefix

### T-014 · Clean the mixer
- status: open
- note (2026-11-15, baker): added   
