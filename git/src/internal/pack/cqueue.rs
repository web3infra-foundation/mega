/// A circular queue data structure that holds elements of generic type `T`.
#[derive(Debug)]
#[allow(unused)]
pub struct CircularQueue<T> {
    data: Vec<Option<T>>, // Storage for the queue elements
    cap: usize, // Capacity of the queue
    write_index: usize, // Index for writing elements
    read_index: usize,  // Index for reading elements
}
#[allow(unused)]
impl<T> CircularQueue<T> {
    /// Creates a new circular queue with the specified capacity.
    pub fn new(cap: usize) -> Self {
        let mut data=  Vec::<Option<T>>::with_capacity(cap+1);
        for _ in 0..=cap{
            data.push(None);
        }
        Self {
            data, // Initialize the Vec with capacity
            cap,
            write_index: 0,
            read_index: 0,
        }
    }

    /// Enqueues an element into the circular queue.
    /// Returns `Ok(())` on success, or `Err("Queue is full")` if the queue is full.
    pub fn enqueue(&mut self, value: T) -> Result<(), &'static str> {
        if self.is_full() {
            Err("Queue is full")
        } else {
            self.data[self.write_index] = Some(value);
            self.write_index = (self.write_index + 1) % (self.cap+1);
            Ok(())
        }
    }
    #[allow(unused)]
    pub fn enequeue_force(&mut self, value: T)  {
        if self.is_full(){
            self.read_index = (self.read_index + 1) % (self.cap+1);
        }
        {self.data[self.write_index].take();}
        self.data[self.write_index]=Some(value);
        self.write_index = (self.write_index + 1) % (self.cap+1);
        
    }

    /// Dequeues an element from the circular queue.
    /// Returns `Some(T)` if the queue is not empty, or `None` if the queue is empty.
    pub fn dequeue(&mut self) -> Option<T> {
        if self.is_empty() {
            None
        } else {
            let value = self.data[self.read_index].take();
            self.read_index = (self.read_index + 1) % (self.cap+1);
            value
        }
    }

    /// Returns a reference to the front element of the circular queue.
    #[allow(unused)]
    pub fn front(&self) -> Option<&T> {
        self.data[self.read_index].as_ref()
    }
    #[allow(unused)]
    pub fn tail(&self) -> Option<&T> {
        self.data[(self.write_index-1)%(self.cap+1)].as_ref()
    }
    /// Checks if the circular queue is empty.
    pub fn is_empty(&self) -> bool {
        self.read_index == self.write_index
    }

    /// Checks if the circular queue is full.
    pub fn is_full(&self) -> bool {
        (self.write_index+1) % (self.cap+1) == self.read_index
    }
}


#[cfg(test)]
mod tests {
    use crate::internal::pack::cqueue::CircularQueue;

    #[test]
    fn test_en_queue() {
        let mut circular_queue = CircularQueue::new(100);

        for i in 0..100 {
            let result = circular_queue.enqueue(i);
            result.unwrap();
        }
        if circular_queue.enqueue(100).is_ok() {
            panic!("this should not be succeedd") ;
        };
    }

    #[test]
    fn test_circular_queue_order() {
        let mut circular_queue = CircularQueue::new(3);

        for i in 1..4 {
            circular_queue.enqueue(i).unwrap();

        }
        //tail 3 2 1 front 
        assert!(circular_queue.is_full());
        circular_queue.dequeue(); // tail 3 2  front 
        let result = circular_queue.front();
        assert_eq!(result, Some(&2));

        circular_queue.enqueue(2).unwrap();// tail 2 3 2  front 
        let result = circular_queue.front();
        assert_eq!(result, Some(&2));

    }

    #[test]
    fn test_force_en_queue(){
        let mut circular_queue = CircularQueue::new(10);
        for i in 1..=20 {
            circular_queue.enequeue_force(i);
            //println!("{:?}",circular_queue)
        }
        let front= circular_queue.front();
        assert_eq!(front,Some(&11));
        let tail = circular_queue.tail();
        assert_eq!(tail,Some(&20));

        for i in 21..=35{
            circular_queue.enequeue_force(i);
        }
        let front= circular_queue.front();
        assert_eq!(front,Some(&26));
        let tail = circular_queue.tail();
        assert_eq!(tail,Some(&35));
    }
}