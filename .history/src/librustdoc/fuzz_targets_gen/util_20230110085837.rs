/// 栈的内部节点
#[derive(Debug)]
struct Node<T> {
    data: T,
    next: Option<Box<Node<T>>>,
}

impl<T> Node<T> {
    fn new(data: T) -> Self {
        Node { data, next: None }
    }

    fn get_last_node(&mut self) -> &mut Self {
        if let Some(ref mut node) = self.next {
            return node.get_last_node();
        }
        self
    }
}

//栈结构
#[derive(Debug, Copy, Clone)]
struct Stack<T> {
    data: Option<Box<Node<T>>>,
    length: usize,
}

impl<T: Copy> Stack<T> {
    fn new() -> Self {
        Stack { data: None, length: 0 }
    }
    fn push(&mut self, data: T) {
        let mut new_node = Node::new(data);
        // push head
        if self.data.is_some() {
            let head = self.data.take();
            new_node.next = head;
            self.data = Some(Box::new(new_node));
        } else {
            self.data = Some(Box::new(new_node));
        }
        self.length += 1
    }
    fn pop(&mut self) -> Option<T> {
        if let Some(ref mut head) = self.data {
            self.length -= 1;
            let data = head.data;
            self.data = head.next.take();
            return Some(data);
        }
        None
    }
    fn length(&self) -> usize {
        self.length
    }
}
