Imagine you're having a vector and now you want to mutate this vector,
it's nothing if this single thread. but imagine we're having a mutlible thread mutation to this
vector, we probably would use mutex here but this is good until you have alot of muatations at the
same  time, then it's not good at all hence it's blocking the other thread from writing until this
operation of muration gets done, so the other muations can't be done until this one is done.

Now if we have a lock free implementation we can have multiple threads writing to the same vector
to the same index, and they can be done concurrently, so we have no need for locks here.

we are using Epoch-Based Reclamation (EBR)

also the memory model 
- acquire
- release
- acquire_release
- relaxed
- squential_consistent

when write to the memory the other threads are unable to see the mutaions you made till you release
them then they will not be able to see it until they acquire it then they can see it.

🔹 The Idea of Epoch-Based Reclamation (EBR)

Think of time split into epochs (like global clock ticks).

Each thread announces which epoch it’s working in.

When a thread wants to free memory:

It retire(s) the object → puts it in a "garbage list" tagged with the current epoch.

Actual freeing happens only when we know:

All active threads have moved past the epoch where the object was retired.

This ensures no thread is still touching it.



