package event

import (
	"sync"
	"sync/atomic"
	"testing"
	"time"
)

func TestDispatcher_Basic(t *testing.T) {
	var count atomic.Int32
	var wg sync.WaitGroup

	handler := func(e int) {
		count.Add(1)
		wg.Done()
	}

	d := NewDispatcher(handler, 2, 10)
	d.Start()
	defer d.Stop()

	// Dispatch 5 events
	wg.Add(5)
	for i := 0; i < 5; i++ {
		if !d.Dispatch(i) {
			t.Error("Dispatch should succeed")
		}
	}

	// Wait for all events to be processed
	wg.Wait()

	if count.Load() != 5 {
		t.Errorf("Expected 5 events processed, got %d", count.Load())
	}
}

func TestDispatcher_QueueFull(t *testing.T) {
	// Handler that blocks
	blocker := make(chan struct{})
	handler := func(e int) {
		<-blocker
	}

	d := NewDispatcher(handler, 1, 2) // 1 worker, queue of 2
	d.Start()
	defer func() {
		close(blocker)
		d.Stop()
	}()

	// First event blocks the worker
	d.Dispatch(1)
	time.Sleep(10 * time.Millisecond)

	// Fill the queue
	d.Dispatch(2)
	d.Dispatch(3)

	// This should fail (queue full)
	if d.Dispatch(4) {
		t.Error("Dispatch should fail when queue is full")
	}
}

func TestDispatcher_Stop(t *testing.T) {
	var count atomic.Int32
	handler := func(e int) {
		count.Add(1)
	}

	d := NewDispatcher(handler, 2, 10)
	d.Start()

	// Dispatch some events
	for i := 0; i < 5; i++ {
		d.Dispatch(i)
	}

	// Stop should wait for events to be processed
	d.Stop()

	// All events should be processed
	if count.Load() != 5 {
		t.Errorf("Expected 5 events processed after stop, got %d", count.Load())
	}
}

func TestDispatcher_NilHandler(t *testing.T) {
	d := NewDispatcher[int](nil, 2, 10)
	d.Start()
	defer d.Stop()

	// Should not panic with nil handler
	d.Dispatch(1)
	time.Sleep(10 * time.Millisecond)
}

func TestDispatcher_MultipleStart(t *testing.T) {
	var count atomic.Int32
	handler := func(e int) {
		count.Add(1)
	}

	d := NewDispatcher(handler, 2, 10)
	d.Start()
	d.Start() // Should be safe to call multiple times
	d.Start()
	defer d.Stop()

	d.Dispatch(1)
	time.Sleep(10 * time.Millisecond)

	// Should only have processed once (not 3x due to multiple starts)
	if count.Load() != 1 {
		t.Errorf("Expected 1 event, got %d", count.Load())
	}
}

func TestDispatcher_QueueLength(t *testing.T) {
	blocker := make(chan struct{})
	handler := func(e int) {
		<-blocker
	}

	d := NewDispatcher(handler, 1, 10)
	d.Start()
	defer func() {
		close(blocker)
		d.Stop()
	}()

	// Block the worker
	d.Dispatch(0)
	time.Sleep(10 * time.Millisecond)

	// Add events to queue
	d.Dispatch(1)
	d.Dispatch(2)
	d.Dispatch(3)

	if d.QueueLength() != 3 {
		t.Errorf("Expected queue length 3, got %d", d.QueueLength())
	}
}
