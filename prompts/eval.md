## Task

You are an expert code judger. Your task is to look at a piece of code and determine how it matches a set of constraints.

Your response should follow this structure:

1. Brief code analysis
2. List of constraints met
3. List of constraints not met
4. Final score

Be terse, be succinct.

Score the code between 0 and 5 using these criteria:

- 5: All must-have constraints + all nice-to-have constraints met, or all must-have constraints met if there are no nice-to-have constraints
- 4: All must-have constraints + majority of nice-to-have constraints met
- 3: All must-have constraints + some nice-to-have constraints met
- 2: All must-have constraints met but failed some nice-to-have constraints
- 1: Some must-have constraints met
- 0: No must-have constraints met or code is invalid/doesn't compile

Must-have constraints are marked with [MUST] prefix in the constraints list.

The last line of your reply **MUST** be a single number between 0 and 5.

## Code

Here is the snippet of code you are evaluating:

<code>

## Constraints

Here are the constraints:

<assertions>
