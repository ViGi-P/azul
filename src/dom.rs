use app_state::AppState;
use traits::LayoutScreen;
use std::collections::BTreeMap;
use id_tree::{NodeId, Children, Arena, FollowingSiblings};
use webrender::api::ItemTag;
use std::sync::{Arc, Mutex};

/// This is only accessed from the main thread, so it's safe to use
pub(crate) static mut NODE_ID: u64 = 0;
pub(crate) static mut CALLBACK_ID: u64 = 0;

pub enum Callback<T: LayoutScreen> {
    /// One-off function (for ex. exporting a file)
    ///
    /// This is best for actions that can run in the background
    /// and you don't need to get updates. It uses a background
    /// thread and therefore the data needs to be sendable.
    Async(fn(Arc<Mutex<AppState<T>>>) -> ()),
    /// Same as the `FnOnceNonBlocking`, but it blocks the current
    /// thread and does not require the type to be `Send`.
    Sync(fn(&mut AppState<T>) -> ()),
}

impl<T: LayoutScreen> Clone for Callback<T> 
{
    fn clone(&self) -> Self {
        match *self {
            Callback::Async(ref f) => Callback::Async(f.clone()),
            Callback::Sync(ref f) => Callback::Sync(f.clone()),
        }
    }
}

/// List of allowed DOM node types
///
/// The reason for this is because the markup5ever crate has
/// special macros for these node types, so either I need to expose the
/// whole markup5ever crate to the end user or I need to build a
/// wrapper type
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NodeType {
    Div,
    Button,
    Ul,
    Li,
    Ol,
    Label,
    Input,
    Form,
    Text { content: String },
}

impl NodeType {
    pub fn get_css_id(&self) -> &'static str {
        use self::NodeType::*;
        match *self {
            Div => "div",
            Button => "button",
            Ul => "ul",
            Li => "li",
            Ol => "ol",
            Label => "label",
            Input => "input",
            Form => "form",
            Text { .. } => "p",
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum On {
    MouseOver,
    MouseDown,
    MouseUp,
    MouseEnter,
    MouseLeave,
    DragDrop,
}

#[derive(Clone)]
pub(crate) struct NodeData<T: LayoutScreen> {
    /// `div`
    pub node_type: NodeType,
    /// `#main`
    pub id: Option<String>,
    /// `.myclass .otherclass`
    pub classes: Vec<String>,
    /// `onclick` -> `my_button_click_handler`
    pub events: CallbackList<T>,
    /// Tag for hit-testing
    pub tag: Option<(u64, u16)>,
}

impl<T: LayoutScreen> CallbackList<T> {
    fn special_clone(&self) -> Self {
        Self {
            callbacks: self.callbacks.clone(),
        }
    }
}

impl<T: LayoutScreen> NodeData<T> {
    pub fn new(node_type: NodeType) -> Self {
        Self {
            node_type: node_type,
            id: None,
            classes: Vec::new(),
            events: CallbackList::<T>::new(),
            tag: None,
        }
    }

    fn special_clone(&self) -> Self {
        Self {
            node_type: self.node_type.clone(),
            id: self.id.clone(),
            classes: self.classes.clone(),
            events: self.events.special_clone(),
            tag: self.tag.clone(),
        }
    }
}

#[derive(Clone)]
pub struct Dom<T: LayoutScreen> {
    pub(crate) arena: Arena<NodeData<T>>,
    pub(crate) root: NodeId,
    pub(crate) current_root: NodeId,
    pub(crate) last: NodeId,
}

#[derive(Clone)]
pub struct CallbackList<T: LayoutScreen> {
    pub(crate) callbacks: BTreeMap<On, Callback<T>>
}

impl<T: LayoutScreen> CallbackList<T> {
    pub fn new() -> Self {
        Self {
            callbacks: BTreeMap::new(),
        }
    }
}

impl<T: LayoutScreen> Dom<T> {
    
    /// Creates an empty DOM
    pub fn new(node_type: NodeType) -> Self {
        let mut arena = Arena::new();
        let root = arena.new_node(NodeData::new(node_type));
        Self {
            arena: arena,
            root: root,
            current_root: root,
            last: root,
        }
    }

    #[inline]
    pub fn add_child(mut self, child: Self) -> Self {
        for ch in child.children() {
            let new_last = self.arena.new_node(child.arena[ch].data.special_clone());
            self.last.append(new_last, &mut self.arena);
            self.last = new_last;
        }
        self
    }

    #[inline]
    pub fn add_sibling(mut self, sibling: Self) -> Self {
        let new_sibling = self.arena.new_node(sibling.arena[sibling.root].data.special_clone());
        self.current_root.append(new_sibling, &mut self.arena);
        self.current_root = new_sibling;
        self
    }

    #[inline]
    pub fn id<S: Into<String>>(mut self, id: S) -> Self {
        self.arena[self.last].data.id = Some(id.into());
        self
    }

    #[inline]
    pub fn class<S: Into<String>>(mut self, class: S) -> Self {
        self.arena[self.last].data.classes.push(class.into());
        self
    }

    #[inline]
    pub fn event(mut self, on: On, callback: Callback<T>) -> Self {
        self.arena[self.last].data.events.callbacks.insert(on, callback);
        self.arena[self.last].data.tag = Some(unsafe { (NODE_ID, 0) });
        unsafe { NODE_ID += 1; };
        self
    }

    fn children(&self) -> Children<NodeData<T>> {
        self.root.children(&self.arena)
    }

    fn following_siblings(&self) -> FollowingSiblings<NodeData<T>> {
        self.root.following_siblings(&self.arena)
    }
}

impl<T: LayoutScreen> Dom<T> {
    
    pub(crate) fn collect_callbacks(&self, callback_list: &mut WrCallbackList<T>, nodes_to_callback_id_list: &mut  BTreeMap<ItemTag, BTreeMap<On, u64>>) {

    }

/*
    pub(crate) fn into_node_ref(self, callback_list: &mut WrCallbackList<T>, nodes_to_callback_id_list: &mut BTreeMap<ItemTag, BTreeMap<On, u64>>) -> NodeRef {

        use std::cell::RefCell;
        use std::collections::HashMap;
        use kuchiki::{NodeRef, Attributes, NodeData, ElementData};

        let mut event_list = BTreeMap::<On, u64>::new();
        let mut attributes = HashMap::new();

        if let Some(id) = self.id {
            attributes.insert(HTML_ID, id);
        }

        for class in self.classes {
            attributes.insert(HTML_CLASS, class);
        }

        for (key, value) in self.events.callbacks {
            unsafe {
                event_list.insert(key, CALLBACK_ID);
                callback_list.callback_list.insert(CALLBACK_ID, value);
                CALLBACK_ID += 1;
            }
        }

        if !event_list.is_empty() {
            use std::mem::transmute;
            nodes_to_callback_id_list.insert(unsafe { (NODE_ID, 0) }, event_list);
            unsafe { NODE_ID += 1; }
            let bytes: [u8; 8] = unsafe { transmute(NODE_ID.to_be()) };
            let bytes_string = unsafe { String::from_utf8_unchecked(bytes.to_vec()) };
            attributes.insert(HTML_NODE_ID, bytes_string);
        }

        let node = match self.node_type {
            NodeType::Text { content } => {
                NodeData::Text(RefCell::new(content))
            },
            _ => {
                NodeData::Element(ElementData {
                    name: QualName::new(None, ns!(html), self.node_type.into()),
                    attributes: RefCell::new(Attributes { map: attributes }),
                    template_contents: None,
                })
            }
        };

        let node = NodeRef::new(node);

        for child in self.children {
            let child_node = child.into_node_ref(callback_list, nodes_to_callback_id_list);
            node.append(child_node);
        }

        node
    }
*/
}


// callbacks

pub struct WebRenderIdList {
    /// Node tag -> List of callback IDs
    pub(crate) callbacks: Option<(ItemTag, BTreeMap<On, u64>)>,
}

impl WebRenderIdList {
    pub fn new() -> Self {
        Self {
            callbacks: None,
        }
    }
}

pub struct WrCallbackList<T: LayoutScreen> {
    /// callback ID -> function pointer
    pub(crate) callback_list: BTreeMap<u64, fn(&mut AppState<T>) -> ()>,
}

impl<T: LayoutScreen> WrCallbackList<T> {
    pub fn new() -> Self {
        Self {
            callback_list: BTreeMap::new(),
        }
    }
}