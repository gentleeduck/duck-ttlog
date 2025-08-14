1. i replaced the vec to be deque
2. added take_snapshot         // replace deque with a fresh empty one to avoid per-element pop_front overhead
3. when panicking the whole application stops and the message is not received and lost
 
