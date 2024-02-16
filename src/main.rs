use once_cell::sync::Lazy;

use salvo::prelude::*;
use salvo::size_limiter;

// use self::models::*;: Importa todos los elementos 
//(estructuras, funciones, etc.) desde el módulo models del mismo archivo.
use self::models::*;

/*
establece una variable estática llamada STORE 
que contiene un Lazy inicializado con una instancia de Db (un Mutex<Vec<Todo>>).
La utilización de Lazy asegura que la inicialización del almacenamiento se realice de manera diferida, es decir, 
solo cuando sea necesario, evitando así la inicialización innecesaria
*/
static STORE: Lazy<Db> = Lazy::new(new_store);



#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();
    start_server().await;
}

pub(crate) async fn start_server() {
    let acceptor = TcpListener::new("127.0.0.1:8080").bind().await;
    Server::new(acceptor).serve(route()).await;
}

fn route() ->Router {
    Router::with_path("todos")
    .hoop(size_limiter::max_size(1024 * 16))
    .get(list_todos)
    .post(create_todo)
    .push(Router::with_path("<id>").put(update_todo).delete(delete_todo))
}

#[handler]
pub async fn list_todos(req: &mut Request, res: &mut Response) {
    //Esta línea parsea el cuerpo de la solicitud
    let opts = req.parse_body::<ListOptions>().await.unwrap_or_default();

    //todos se convierte en un MutexGuard, que es un tipo que garantiza la exclusión mutua.
    let todos = STORE.lock().await;
    // A partir de aca, clonamos el contenido del vector, lo convertimos en un iterable, luego hace algunas cosas para la paginacion
    // collect -> agarra los elementos restante y los guarda en un nuevo vector.
    let todos: Vec<Todo> = todos
    .clone()
    .into_iter()
    .skip(opts.offset.unwrap_or(0))
    .take(opts.limit.unwrap_or(std::usize::MAX))
    .collect();
    // renderizamos en un json el nuevo vector 
    res.render(Json(todos));
    
}

#[handler]
pub async fn create_todo(req: &mut Request, res: &mut Response) {
    let new_todo = req.parse_body::<Todo>().await.unwrap();
    // linea que registra mensajes de depuracion
    tracing::debug!(todo = ?new_todo, "create_todo");

    let mut vec = STORE.lock().await;

    //iteramos sobre el vector vec
    for todo in vec.iter() {
        //si coincide el id del nuevo vector con uno ya existente damos un aviso de bad request
        if todo.id == new_todo.id {
            tracing::debug!(id = ?new_todo.id, "todo is already exists");
            res.status_code(StatusCode::BAD_REQUEST);
            return;
        }
    }
    // se agrega la nueva posicion al vector
    vec.push(new_todo);
    // status code de creado
    res.status_code(StatusCode::CREATED);
}


#[handler]
pub async fn update_todo(req: &mut Request, res: &mut Response) {
    // id de los parametros
    let id = req.param::<i64>("id").unwrap();
    // extrae y parsea el cuerpo de la solicitud y se le indica que espera un obj Todo
    let updated_todo = req.parse_body::<Todo>().await.unwrap();
    tracing::debug!(todo = ?updated_todo, id = ?id, "update todo");

    let mut vec = STORE.lock().await;

    // itera sobre el vector permitiendo mutabilidad
    for todo in vec.iter_mut() {
        if todo.id == id {
            // si coincide el id, lo actualiza todo accediendo a la memoria
            *todo = updated_todo;
            res.status_code(StatusCode::OK);
            return ;
        }
    }

    tracing::debug!(?id, "todo is not found");
    res.status_code(StatusCode::NOT_FOUND);

}

#[handler]
pub async fn delete_todo(req: &mut Request, res: &mut Response) {
    // id de parametros
    let id = req.param::<i64>("id").unwrap();
    // mensaje de depuracion
    tracing::debug!(?id, "delete todo");

    let mut vec = STORE.lock().await;

    // sacamos el len del vector
    let len = vec.len();
    // modificamos el vector actual 
    // |todo| -> argumento closure --> representa cada tarea en el vector
    // todo.id != id --> si el todo.id no es igual al id del param y quiere decir que si coinciden devuelve un false y elimina la posicion del vector
    vec.retain(|todo| todo.id != id);

    // compara la longitud del vector para saber si se elimino o no y despues devolver un status code
    let deleted = vec.len() != len;
    if deleted  {
        res.status_code(StatusCode::NO_CONTENT);
    } else {
        tracing::debug!(?id, "todo is not found");
        res.status_code(StatusCode::NOT_FOUND);
    }
    
}

mod models {
    /* 
    use serde::{Serialize, Deserialize};: Importa los traits Serialize y Deserialize del paquete serde. Estos traits son utilizados
    para serializar y deserializar estructuras de datos en formatos como JSON.
    */
    use serde::{Serialize, Deserialize};
    /*
    use tokio::sync::Mutex;: Importa el tipo Mutex del paquete tokio. 
    Mutex se utiliza para gestionar el acceso concurrente a datos compartidos.
     */
    use tokio::sync::Mutex;

    /*
    pub type Db = Mutex<Vec<Todo>>;: Define un alias (Db) para Mutex<Vec<Todo>>, que es un mutex que envuelve un vector de Todo. 
    Esto probablemente se utilice como una especie de almacenamiento compartido.
     */
    pub type Db = Mutex<Vec<Todo>>;

    /*
    pub fn new_store() -> Db { ... }: Define una función new_store que devuelve una nueva instancia de Db (Mutex con un vector vacío de Todo). 
    Esta función probablemente se utilizara para inicializar el almacenamiento.
     */
    pub fn new_store() ->Db {
        Mutex::new(Vec::new())
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct Todo {
        pub id: i64, 
        pub text: String,
        pub completed: bool,
    }

    #[derive(Deserialize, Debug, Default)]
    pub struct ListOptions {
        pub offset: Option<usize>,
        pub limit: Option<usize>,
    }
}