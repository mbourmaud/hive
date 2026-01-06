// Package event provides a worker pool-based event dispatcher.
package event

import (
	"context"
	"sync"
)

// Dispatcher handles event dispatch with a bounded worker pool.
// It prevents goroutine leaks by limiting concurrent event handlers.
type Dispatcher[T any] struct {
	handler func(T)
	queue   chan T
	workers int
	wg      sync.WaitGroup
	ctx     context.Context
	cancel  context.CancelFunc
	started bool
	mu      sync.Mutex
}

// NewDispatcher creates a new event dispatcher.
// - workers: number of concurrent workers processing events
// - queueSize: maximum number of events that can be buffered
func NewDispatcher[T any](handler func(T), workers, queueSize int) *Dispatcher[T] {
	if workers <= 0 {
		workers = 4
	}
	if queueSize <= 0 {
		queueSize = 100
	}

	ctx, cancel := context.WithCancel(context.Background())

	return &Dispatcher[T]{
		handler: handler,
		queue:   make(chan T, queueSize),
		workers: workers,
		ctx:     ctx,
		cancel:  cancel,
	}
}

// Start launches the worker pool. Safe to call multiple times.
func (d *Dispatcher[T]) Start() {
	d.mu.Lock()
	defer d.mu.Unlock()

	if d.started {
		return
	}
	d.started = true

	for i := 0; i < d.workers; i++ {
		d.wg.Add(1)
		go d.worker()
	}
}

// worker processes events from the queue.
func (d *Dispatcher[T]) worker() {
	defer d.wg.Done()

	for event := range d.queue {
		if d.handler != nil {
			d.handler(event)
		}
	}
}

// Dispatch sends an event to be processed by a worker.
// If the queue is full, the event is dropped to prevent blocking.
// Returns true if the event was queued, false if dropped.
func (d *Dispatcher[T]) Dispatch(event T) bool {
	select {
	case d.queue <- event:
		return true
	default:
		// Queue is full, drop the event to prevent blocking
		return false
	}
}

// DispatchBlocking sends an event and blocks until it's queued.
// Use this when events must not be dropped.
func (d *Dispatcher[T]) DispatchBlocking(event T) {
	d.queue <- event
}

// Stop gracefully shuts down the dispatcher.
// Waits for all queued events to be processed.
func (d *Dispatcher[T]) Stop() {
	d.mu.Lock()
	if !d.started {
		d.mu.Unlock()
		return
	}
	d.mu.Unlock()

	d.cancel()
	close(d.queue)
	d.wg.Wait()
}

// QueueLength returns the current number of events in the queue.
func (d *Dispatcher[T]) QueueLength() int {
	return len(d.queue)
}
