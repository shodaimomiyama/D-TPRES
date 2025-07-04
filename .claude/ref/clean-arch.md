## 3.2. ドメイン層の実装

  

## ¶

ドメイン層の開発の流れと、役割分担について説明する。

下記の説明では、複数の開発チームが存在する状態でアプリケーションを構築するケースを想定しているが、1チームで開発する場合でも、開発フロー自体は変わらない。

> ![implementation flow of domain layer](https://terasolunaorg.github.io/guideline/current/ja/_images/service_implementation_flow.png)
> 
> implementation flow of domain layer
> 
> | 項番 | 担当チーム | 説明 |
> | --- | --- | --- |
> | (1) | 共通開発チーム | 共通開発チームは、Entityクラスの設計およびEntityクラスの作成を行う。 |
> | (2) | 共通開発チーム | 共通開発チームは、(1)で抽出したEntityクラスに対するアクセスパターンを整理し、Repositoryインタフェースのメソッド設計を行う。  複数の開発チームで共有するメソッドに対する実装については、共通開発チームで実装することが望ましい。 |
> | (3) | 共通開発チーム | 共通開発チームは、(1)と(2)で作成したEntityクラスと、Repositoryを業務アプリケーション開発チームに提供する。  このタイミングで、各業務アプリケーション開発チームに対して、Repositoryインタフェースの実装を依頼する。 |
> | (4) | 業務アプリケーション開発チーム | 業務アプリケーション開発チームは、自チーム担当分のRepositoryインタフェースの実装を行う。 |
> | (5) | 業務アプリケーション開発チーム | 業務アプリケーション開発チームは、共通開発チームから提供されたEntityクラスおよびRepositoryと自チームで作成したRepositoryを利用して、ServiceインタフェースおよびServiceクラスの実装を行う。 |
> 
> Warning
> 
> 開発規模が大きいシステムでは、アプリケーションを複数のチームに分担して開発を行う場合がある。その場合は、EntityクラスおよびRepositoryを設計するための共通チームを設けることを強く推奨する。
> 
> 共通チームを設ける体制が組めない場合は、EntityクラスおよびRepositoryを作成せずに、ServiceからO/R Mapper(MyBatisなど)を直接呼び出して、業務データにアクセスする方法を採用することを検討すること。

  

## ¶

### ¶

Entityは原則以下の方針で作成する。

具体的な作成方法については、で示す。

> | 項番 | 方針 | 補足 |
> | --- | --- | --- |
> | 1. | Entityクラスは、テーブル毎に作成する。 | ただし、テーブル間の関連を保持するためのマッピングテーブルについては、Entityクラスは不要である。  また、テーブルが正規化されていない場合は、必ずしもテーブル毎にはならない。テーブルが正規化されていない時のアプローチは、を参照されたい。 |
> | 2. | テーブルにFK(Foreign Key)がある場合は、FK先のテーブルのEntityクラスをプロパティとして定義する。 | FK先のテーブルとの関係が、1:Nになる場合は、 `java.util.List<E>` または `java.util.Set<E>` のどちらかを使用する。  FK先のテーブルに対応するEntityのことを、本ガイドライン上では、関連Entityと呼ぶ。 |
> | 3. | コード系テーブルは、Entityとして扱うのではなく、 `java.lang.String` などの基本型で扱う。 | コード系テーブルとは、コード値と、コード名のペアを管理するためのテーブルのことである。  コード値によって処理分岐する必要がある場合は、コード値に対応するenumクラスを作成し、作成したenumをプロパティとして定義することを推奨する。 |

> Warning
> 
> テーブルが正規化されていない場合は、 以下の点を考慮して **EntityクラスおよびRepositoryを作成する方式を採用すべきか検討した方がよい。** 特に正規化されていないテーブルとJPAとの相性はあまりよくないので、テーブルが正規化されていない場合は、JPAを使用してEntityオブジェクトを操作する方式は採用しない方が無難である。
> 
> - Entityを作成する難易度が高くなるため、適切なEntityクラスの作成が出来ない可能性がある。
> 	加えて、Entityクラスを作成するために、必要な工数が多くなる可能性も高い。
> 	前者は、「適切に正規化できるエンジニアをアサインできるか？」という観点、後者は、「工数をかけて正規化されたEntityクラスを作成する価値があるか？」という観点で、検討することになる。
> - 業務データにアクセスする際の処理として、Entityクラスとテーブルの構成の差分を埋めるための処理が、必要となる。
> 	これは、「工数をかけて、Entityとテーブルの差分を埋めるための処理を実装する価値があるか？」という観点で検討することになる。
> 
> EntityクラスとRepositoryを作成する方式を採用することを推奨するが、作成するアプリケーションの特性、 プロジェクトの特性(開発体制や開発プロセスなど)を加味して、採用する構成を決めて頂きたい。

> Note
> 
> テーブルは正規化されていないが、アプリケーションとして、正規化されたEntityとして業務データを扱いたい場合は、インフラストラクチャ層のRepositoryImplの実装として、MyBatisを採用することを推奨する。
> 
> MyBatisは、データベースで管理されているレコードとオブジェクトをマッピングするという考え方ではなく、SQLとオブジェクトをマッピングという考え方で開発されたO/R Mapperであるため、SQLの実装次第で、テーブル構成に依存しないオブジェクトへのマッピングができる。

  

### ¶

Entityクラスの作成方法を、具体例を用いて説明する。

以下は、ショッピングサイトで商品を購入する際に必要となる業務データを、Entityクラスとして作成する例となっている。

  

#### ¶

商品を購入する際に必要となる業務データを保持するテーブルは、以下の構成となっている。

> ![Example of table layout](https://terasolunaorg.github.io/guideline/current/ja/_images/service_entity_table_layout.png)
> 
> Example of table layout
> 
> | 項番 | 分類 | テーブル名 | 説明 |
> | --- | --- | --- | --- |
> | (1) | トランザクション系 | t\_order | 注文を保持するテーブル。1つの注文に対して1レコードが格納される。 |
> | (2) |  | t\_order\_item | 1つの注文で購入された商品を保持するテーブル。1つの注文で複数の商品が購入された場合は商品数分レコードが格納される。 |
> | (3) |  | t\_order\_coupon | 1つの注文で使用されたクーポンを保持するテーブル。1つの注文で複数のクーポンが使用された場合はクーポン数分レコードが格納される。クーポンを使用しなかった場合、レコードは格納されない。 |
> | (4) | マスタ系 | m\_item | 商品を定義するマスタテーブル。 |
> | (5) |  | m\_category | 商品のカテゴリを定義するマスタテーブル。 |
> | (6) |  | m\_item\_category | 商品が所属するカテゴリを定義するマスタテーブル。商品とカテゴリのマッピングを保持している。1つの商品は複数のカテゴリに属すことができるモデルとなっている。 |
> | (7) |  | m\_coupon | クーポンを定義するマスタテーブル。 |
> | (8) | コード系 | c\_order\_status | 注文ステータスを定義するコードテーブル。 |

  

#### ¶

上記テーブルから作成方針に則ってEntityクラスを作成すると、以下のような構成となる。

> ![Example of entity layout](https://terasolunaorg.github.io/guideline/current/ja/_images/service_entity_entity_layout.png)
> 
> Example of entity layout
> 
> | 項番 | クラス名 | 説明 |
> | --- | --- | --- |
> | (1) | Order | t\_orderテーブルの1レコードを表現するEntityクラス。  関連Entityとして、 `OrderItem` および `OrderCoupon` を複数保持する。 |
> | (2) | OrderItem | t\_order\_itemテーブルの1レコードを表現するEntityクラス。  関連Entityとして、 `Item` を保持する。 |
> | (3) | OrderCoupon | t\_order\_couponテーブルの1コードを表現するEntityクラス。  関連Entityとして、 `Coupon` を保持する。 |
> | (4) | Item | m\_itemテーブルの1コードを表現するEntityクラス。  関連Entityとして、所属している `Category` を複数保持する。 `Item` と `Category` の紐づけは、m\_item\_categoryテーブルによって行われる。 |
> | (5) | Category | m\_categoryテーブルの1レコードを表現するEntityクラス。 |
> | (6) | ItemCategory | m\_item\_categoryテーブルは、m\_itemテーブルとm\_categoryテーブルとの関連を保持するためのマッピングテーブルなので、Entityクラスは作成しない。 |
> | (7) | Coupon | m\_couponテーブルの1レコードを表現するEntityクラス。 |
> | (8) | OrderStatus | c\_order\_statusテーブルはコード系テーブルなので、Entityクラスは作成しない。 |

上記のエンティティ図をみると、ショッピングサイトのアプリケーションとして主体のEntityクラスとして扱われるのは、Orderクラスのみと思ってしまうかもしれないが、主体となる得るEntityクラスはOrderクラス以外にも存在する。

以下に、主体のEntityとしてなり得るEntityと、主体のEntityにならないEntityを分類する。

> ![Example of entity layout](https://terasolunaorg.github.io/guideline/current/ja/_images/service_entity_entity_class_layout.png)
> 
> Example of entity layout

  

ショッピングサイトのアプリケーションを作成する上で、主体のEntityとしてなり得るのは、以下4つである。

> | 項番 | Entityクラス | 主体のEntityとなる得る理由 |
> | --- | --- | --- |
> | (1) | Orderクラス | ショッピングサイトにおいて、最も重要な主体となるEntityクラスのひとつである。  Orderクラスは、注文そのものを表現するEntityであり、Orderクラスなくしてショッピングサイトを作成することはできない。 |
> | (2) | Itemクラス | ショッピングサイトにおいて、最も重要な主体となるEntityクラスのひとつである。  Itemクラスは、ショッピングサイトで扱っている商品そのものを表現するEntityであり、Itemクラスなくしてショッピングサイトを作成することはできない。 |
> | (3) | Categoryクラス | 一般的なショッピングサイトでは、トップページや共通的メニューとして、サイトで扱っている商品のカテゴリを表示している。  このようなショッピングサイトのアプリケーションでは、Categoryクラスを主体のEntityとして扱うことになる。カテゴリの一覧検索などの処理が想定される。 |
> | (4) | Couponクラス | ショッピングサイトにおいて、商品の販売促進を行う手段としてクーポンによる値引きを行うことがある。  このようなショッピングサイトのアプリケーションでは、Couponクラスを主体のEntityとして扱うことなる。クーポンの一覧検索などの処理が想定される。 |

ショッピングサイトのアプリケーションを作成する上で、主体のEntityとならないのは、以下2つである。

> | 項番 | Entityクラス | 主体のEntityにならない理由 |
> | --- | --- | --- |
> | (5) | OrderItemクラス | このクラスは、1つの注文で購入された商品1つを表現するクラスであり、Orderクラスの関連Entityとしてのみ存在するクラスとなる。  そのため、OrderItemクラスが、主体のEntityとして扱われることは原則ない。 |
> | (6) | OrderCoupon | このクラスは、1つの注文で使用されたクーポン1つを表現するクラスであり、Orderクラスの関連Entityとしてのみ存在するクラスとなる。  そのため、OrderCouponクラスが主体のEntityとして扱われることは原則ない。 |

  

## ¶

### ¶

Repositoryは、以下2つの役割を担う。

1. **Serviceに対して、Entityのライフサイクルを制御するための操作（Repositoryインタフェース）を提供する。**
	Entityのライフサイクルを制御するための操作は、EntityオブジェクトへのCRUD操作となる。

> ![provide access operations to entity](https://terasolunaorg.github.io/guideline/current/ja/_images/repository_responsibility_1.png)
> 
> provide access operations to entity

1. **Entityを永続化する処理(Repositoryインタフェースの実装クラス)を提供する。**
	Entityオブジェクトは、アプリケーションのライフサイクル(サーバの起動や、停止など)に依存しないレイヤに、永続化しておく必要がある。
	Entityの永続先は、リレーショナルデータベースになることが多いが、NoSQLデータベース、キャッシュサーバ、外部システム、ファイル（共有ディスク）などになることもある。
	実際の永続化処理は、O/R Mapperなどから提供されているAPIを使って行う。
	この役割は、インフラストラクチャ層のRepositoryImplで実装することになる。詳細については、 [インフラストラクチャ層の実装](https://terasolunaorg.github.io/guideline/current/ja/ImplementationAtEachLayer/InfrastructureLayer.html) を参照されたい。

> ![persist entity](https://terasolunaorg.github.io/guideline/current/ja/_images/repository_responsibility_2.png)
> 
> persist entity

  

### ¶

Repositoryは、RepositoryインタフェースとRepositoryImplで構成され、それぞれ以下の役割を担う。

> ![persist entity](https://terasolunaorg.github.io/guideline/current/ja/_images/repository_classes_responsibility.png)
> 
> persist entity
> 
> | 項番 | クラス(インタフェース) | 役割 | 説明 |
> | --- | --- | --- | --- |
> | (1) | Repositoryインタフェース | 業務ロジック(Service)を実装する上で必要となるEntityのライフサイクルを制御するメソッドを定義する。 | 永続先に依存しないEntityの、CRUD操作用のメソッドを定義する。  Repositoryインタフェースは、業務ロジック(Service)を実装する上で必要となるEntityの操作を定義する役割を担うので、ドメイン層に属することになる。 |
> | (2) | RepositoryImpl | Repositoryインタフェースで定義されたメソッドの実装を行う。 | 永続先に依存したEntityのCRUD操作の実装を行う。実際のCRUD処理は、Spring Framework、O/R Mapper、ミドルウェアなどから提供されている永続処理用のAPIを利用して行う。  RepositoryImplは、Repositoryインタフェースで定義された操作の実装を行う役割を担うので、インフラストラクチャ層に属することになる。  RepositoryImplの実装については、 [インフラストラクチャ層の実装](https://terasolunaorg.github.io/guideline/current/ja/ImplementationAtEachLayer/InfrastructureLayer.html) を参照されたい。 |

永続先が複数になる場合、以下のような構成となる。

以下のような構成を取ることで、Entityの永続先に依存したロジックを、業務ロジック(Service)から排除することができる。

> ![persist entity](https://terasolunaorg.github.io/guideline/current/ja/_images/repository_not_depends_on.png)
> 
> persist entity
> 
> Note
> 
> **永続先に依存したロジックを、Serviceから100%排除できるのか？**
> 
> 永続先の制約や、使用するライブラリの制約などにより、排除できないケースもある。可能な限り、永続先に依存するロジックは、Serviceではなく、RepositoryImplで実装することを推奨するが、永続先に依存するロジックを排除するのが難しい場合や、排除することで得られるメリットが少ない場合は、無理に排除せず、業務ロジック(Service)の処理として、永続先に依存するロジックを実装してもよい。
> 
> 排除できない具体例として、Spring Data JPAから提供されている `org.springframework.data.jpa.repository.JpaRepository` インタフェースのsaveメソッドの呼び出し時に、一意制約エラーをハンドリングしたい場合である。
> 
> JPAではEntityへの操作はキャッシュされ、トランザクションコミット時にSQLを発行する仕組みになっている。そのため、JpaRepositoryのsaveメソッドを呼び出しても、SQLは発行されないので、一意制約違反をロジックでハンドリングすることができない。
> 
> JPAでは、明示的にSQLを発行する手段として、キャッシュされている操作を反映するためのメソッド（flushメソッド）があり、JpaRepositoryではsaveAndFlush、flushというメソッドが同じ目的で提供されている。そのため、Spring Data JPAのJpaRepositoryを使って、一意制約違反エラーをハンドリングする必要がある場合は、JPA依存のメソッド（saveAndFlushや、flush）を呼び出す必要がある。
> 
> Warning
> 
> Repositoryを設ける最も重要な目的は、永続先に依存するロジックを、業務ロジックから排除することではないという点である。
> 
> 最も重要な目的は、業務データへアクセスするための操作をRepositoryへ分離することで、業務ロジック(Service)の実装範囲をビジネスルールに関する実装に専念させるという点である。
> 
> 結果として、永続先に依存するロジックは業務ロジック(Service)ではなく、Repository側に実装される事になる。

  

### ¶

Repositoryは原則以下の方針で作成する。

> | 項番 | 方針 | 補足 |
> | --- | --- | --- |
> | 1. | Repositoryは、主体となるEntityに対して作成する。 | これは、関連Entityを操作するためだけのRepositoryが不要であることを意味する。  ただし、アプリケーションの特性(高い性能要件があるアプリケーションなど)では、関連Entityを操作するためのRepositoryを設けた方が、よい場合もある。 |
> | 2. | Repositoryインタフェースと、RepositoryImplは、基本的にドメイン層の同じパッケージに配置する。 | Repositoryは、Repositoryインタフェースがドメイン層、RepositoryImplがインフラストラクチャ層に属することとなるが、  Javaのパッケージとしては、基本的には、ドメイン層のRepositoryインタフェースと同じパッケージでよい。 |
> | 3. | Repositoryで使用するDTOは、Repositoryインタフェースと同じパッケージに配置する。 | 例えば、検索条件を保持するDTOや、Entityの一部の項目のみを定義したサマリ用のDTOなどがあげられる。 |

  

### ¶

#### ¶

以下にRepositoryインタフェースの作成例を紹介する。

- `SimpleCrudRepository.java`
	このインタフェースは、シンプルなCRUD操作のみを提供している。
	メソッドのシグネチャは、Spring Dataから提供されている `CrudRepository` インタフェースや、 `PagingAndSortingRepository` インタフェースを参考に作成している。
	```java
	public interface SimpleCrudRepository<T, ID extends Serializable> {
	    // (1)
	    T findById(ID id);
	    // (2)
	    boolean existsById(ID id);
	    // (3)
	    List<T> findAll();
	    // (4)
	    Page<T> findAll(Pageable pageable);
	    // (5)
	    long count();
	    // (6)
	    T save(T entity);
	    // (7)
	    void delete(T entity);
	}
	```
	| 項番 | 説明 |
	| --- | --- |
	| (1) | 指定したIDに対応するEntityを、取得するためのメソッド。 |
	| (2) | 指定したIDに対応するEntityが、存在するか判定するためのメソッド。 |
	| (3) | 全てのEntityを取得するためのメソッド。 Spring Dataでは、 `java.util.Iterable` であったが、サンプルとしては、 `java.util.List` にしている。 |
	| (4) | 指定したページネーション情報（取得開始位置、取得件数、ソート情報）に該当するEntityのコレクションを取得するためのメソッド。  `Pageable` インタフェースおよび `Page` インタフェースはSpring Dataより提供されているクラス（インターフェース）である。 |
	| (5) | Entityの総件数を取得するためのメソッド。 |
	| (6) | 指定されたEntityを保存（作成、更新）するためのメソッド。 |
	| (7) | 指定したEntityを、削除するためのメソッド。 |
- `TodoRepository.java`
	下記は、チュートリアルで作成したTodoエンティティのRepositoryを、上で作成した `SimpleCrudRepository` インタフェースベースに作成した場合の例である。
	```java
	// (1)
	public interface TodoRepository extends SimpleCrudRepository<Todo, String> {
	    // (2)
	    long countByFinished(boolean finished);
	}
	```
	| 項番 | 説明 |
	| --- | --- |
	| (1) | エンティティの型を示すジェネリック型「T」にTodoエンティティ、エンティティのID型を示すジェネリック型「ID」にStringクラスを指定することで、  Todoエンティティ用のRepositoryインタフェースが生成される。 |
	| (2) | `SimpleCrudRepository` インタフェースから提供されていないメソッドを追加している。  ここでは、「指定したタスクの終了状態に一致するTodoエンティティの件数を取得するメソッド」を追加している。 |

  

#### ¶

汎用的なCRUD操作を行うメソッドについては、Spring Dataから提供されている `CrudRepository` や、 `PagingAndSortingRepository` と同じシグネチャにすることを推奨する。

ただし、コレクションを返却する場合は、 `java.lang.Iterable` ではなく、ロジックで扱いやすいインタフェース（ `java.util.Collection` や、 `java.util.List` ）でもよい。

実際のアプリケーション開発では、汎用的なCRUD操作のみで開発できることは稀で、かならずメソッドの追加が必要になる。

追加するメソッドは、以下のルールに則り追加することを推奨する。

> | 項番 | メソッドの種類 | ルール |
> | --- | --- | --- |
> |  | 1件検索系のメソッド | 1. メソッド名は、条件に一致するEntityを1件取得するためのメソッドであることを明示するために、 **findBy** で始める。 2. メソッド名のfindBy以降は、検索条件となるフィールドの物理名、または、論理的な条件名などを指定し、どのような状態のEntityが取得されるのか、推測できる名前とする。 3. 引数は、条件となるフィールド毎に用意する。ただし、条件が多い場合は、条件をまとめたDTOを用意してもよい。 4. 返り値は、Entityクラスを指定する。 |
> |  | 複数件検索系のメソッド | 1. メソッド名は、条件に一致するEntityを、すべて取得するためのメソッドであることを明示するために、 **findAllBy** で始める。 2. メソッド名のfindAllBy以降は、検索条件となるフィールドの物理名または論理的な条件名を指定し、どのような状態のEntityが取得されるのか推測できる名前とする。 3. 引数は、条件となるフィールド毎に用意する。ただし、条件が多い場合は、条件をまとめたDTOを用意してもよい。 4. 返り値は、Entityクラスのコレクションを指定する。 |
> |  | 複数件ページ検索系のメソッド | 1. メソッド名は、条件に一致するEntityの該当ページ部分を取得するためのメソッドである事を明示するために、 **findPageBy** で始める。 2. メソッド名のfindPageBy以降は、検索条件となるフィールドの物理名または論理的な条件名を指定し、どのような状態のEntityが取得されるのか推測できる名前とする。 3. 引数は、条件となるフィールド毎に用意する。ただし、条件が多い場合は、条件をまとめたDTOを用意してもよい。ページネーション情報（取得開始位置、取得件数、ソート情報）は、Spring Dataより提供されている `Pageable` インタフェースとすることを推奨する。 4. 返り値は、Spring Dataより提供されている `Page` インタフェースとすることを推奨する。 |
> |  | 件数のカウント系のメソッド | 1. メソッド名は、条件に一致するEntityの件数をカウントするためのメソッドである事を明示するために、 **countBy** で始める。 2. 返り値は、long型にする。 3. メソッド名のcountBy以降は、検索条件となるフィールドの物理名または論理的な条件名を指定し、どのような状態のEntityの件数が取得されるのか推測できる名前とする。 4. 引数は、条件となるフィールド毎に用意する。ただし、条件が多い場合は、条件をまとめたDTOを用意してもよい。 |
> |  | 存在判定系のメソッド | 1. メソッド名は、条件に一致するEntityが存在するかチェックするためのメソッドである事を明示するために、 **existsBy** で始める。 2. メソッド名のexistsBy以降は、検索条件となるフィールドの物理名または論理的な条件名を指定し、どのような状態のEntityの存在チェックを行うのか推測できる名前とする。 3. 引数は、条件となるフィールド毎に用意する。ただし、条件が多い場合は、条件をまとめたDTOを用意してもよい。 4. 返り値は、boolean型にする。 |
> 
> Note
> 
> 更新系のメソッドも、同様のルールに則り、追加することを推奨する。
> 
> findの部分が、updateまたはdeleteとなる。

- `Todo.java` (Entity)
	```java
	public class Todo implements Serializable {
	    private String todoId;
	    private String todoTitle;
	    private boolean finished;
	    private Date createdAt;
	    // omitted
	}
	```

  

- `TodoRepository.java`
	```java
	public interface TodoRepository extends SimpleCrudRepository<Todo, String> {
	    // (1)
	    Todo findByTodoTitle(String todoTitle);
	    // (2)
	    List<Todo> findAllByUnfinished();
	    // (3)
	    Page<Todo> findPageByUnfinished();
	    // (4)
	    long countByExpired(int validDays);
	    // (5)
	    boolean existsByCreateAt(Date date);
	}
	```
	| 項番 | 説明 |
	| --- | --- |
	| (1) | タイトルが一致するTODO(todoTitle=引数で指定した値のTODO)を取得するメソッドの定義例。  findBy以降に、条件となるフィールドの物理名(todoTitle)を指定している。 |
	| (2) | 未完了のTODO(finished=falseのTODO)を全件取得するメソッドの定義例。  findAllBy以降に、論理的な条件名を指定している。 |
	| (3) | 未完了のTODO(finished=falseのTODO)の該当ページ部分を取得するメソッドの定義例。  findPageBy以降に、論理的な条件名を指定している。 |
	| (4) | 完了期限を過ぎたTODO(createdAt < sysdate - 引数で指定した有効日数 && finished=falseのTODO)の件数を取得するメソッドの定義例。  countBy以降に、論理的な条件名を指定している。 |
	| (5) | 指定日に作成されている、TODO(createdAt=指定日)が存在するか判定するメソッドの定義例。  existsBy以降に、条件となるフィールドの物理名(createdAt)を指定している。 |

  

## ¶

### ¶

Serviceは、以下2つの役割を担う。

1. **Controllerに対して業務ロジックを提供する。**
	業務ロジックは、アプリケーションで使用する業務データの参照、更新、整合性チェックおよびビジネスルールに関わる各種処理で構成される。
	業務データの参照および更新処理をRepository(またはO/R Mapper)に委譲し、 **Serviceではビジネスルールに関わる処理の実装に専念することを推奨する。**
	Note
	**ControllerとServiceで実装するロジックの責任分界点について**
	本ガイドラインでは、ControllerとServiceで実装するロジックは、以下のルールに則って実装することを推奨する。
	1. クライアントからリクエストされたデータに対する単項目チェック、相関項目チェックはController側(Bean ValidationまたはSpring Validator)で行う。
	2. Serviceに渡すデータへの変換処理(Bean変換、型変換、形式変換など)は、ServiceではなくController側で行う。
	3. **ビジネスルールに関わる処理はServiceで行う。** 業務データへのアクセスは、RepositoryまたはO/R Mapperに委譲する。
	4. ServiceからControllerに返却するデータ（クライアントへレスポンスするデータ）に対する値の変換処理(型変換、形式変換など)は、Serviceではなく、Controller側（Viewクラスなど）で行う。
	![responsibility of logic](https://terasolunaorg.github.io/guideline/current/ja/_images/service_responsibility-of-logic.png)
	responsibility of logic
2. **トランザクション境界を宣言する。**
	データの一貫性を保障する必要がある処理（主にデータの更新処理）を行う業務ロジックの場合、トランザクション境界を宣言する。
	データの参照処理の場合でも業務要件によっては、トランザクション管理が必要になる場合もあるので、その場合は、トランザクション境界を宣言する。
	**トランザクション境界は、原則Serviceに設ける。** アプリケーション層(Web層)にトランザクション境界が設けられている場合、業務ロジックの抽出が正しく行われていない可能性があるので、見直しを行うこと。
	![transaction boundary](https://terasolunaorg.github.io/guideline/current/ja/_images/service_transaction-boundary.png)
	transaction boundary
	詳細は、を参照されたい。

  

### ¶

> ![class dependency](https://terasolunaorg.github.io/guideline/current/ja/_images/service_class-dependency.png)
> 
> class dependency

  

#### ¶

本ガイドラインでは、Serviceクラスのメソッドから、別のServiceクラスのメソッドを呼び出すことを、原則禁止としている。

これは、Serviceクラスは、特定のControllerに対して業務ロジックを提供するクラスであり、別のServiceから利用される前提で作成しないためである。

仮に、別のServiceクラスから直接呼び出してしまうと、以下のような状況が発生しやすくなり、 **保守性などを低下させる危険性が、高まる。**

> | 項番 | 発生しうる状況 |
> | --- | --- |
> |  | 本来は、呼び出し元のServiceクラスで実装すべきロジックが、処理を一ヶ所にまとめたいという理由などにより、呼び出し先のServiceクラスで実装されてしまう。  その際に、 **呼び出し元を意識するための引数（フラグ）などが、安易に追加され、間違った共通化が行われてしまう。結果として、見通しの悪いモジュール構成になってしまう。** |
> |  | 呼び出し経路やパターンが多くなることで、 **仕様変更や、バグ改修の際のソース修正に対する影響範囲の把握が難しくなる。** |

  

#### ¶

業務ロジックの作りを統一したい場合に、シグネチャを限定するようなインタフェースや、基底クラスを作成することがある。

シグネチャを限定するインタフェースや基底クラスを設けることで、開発者ごとに、作りの違いが発生しないようにする目的もある。

> Note
> 
> 大規模開発において、サービスイン後の保守性等を考慮して業務ロジックの作りを合わせておきたい場合や、開発者のスキルがあまり高くない場合などの状況下では、シグネチャを限定するようなインタフェースを設けることも、選択肢の一つとして考えてもよい。
> 
> 本ガイドラインでは、シグネチャを限定するようなインタフェースを作成することは、特に推奨していないが、プロジェクトの特性を加味して、どのようなアーキテクチャにするか決めて頂きたい。
> 
> Note
> 
> **シグネチャを制限するインタフェースおよび基底クラスの実装サンプル**
> 
> - シグネチャを限定するようなインタフェース
> 	```java
> 	// (1)
> 	public interface BLogic<I, O> {
> 	  O execute(I input);
> 	}
> 	```
> 	| 項番 | 説明 |
> 	| --- | --- |
> 	| (1) | 業務ロジックの実装メソッドのシグニチャを制限するためのインタフェース。  上記例では、入力情報(I)と出力情報(O)の総称型として定義されており、 業務ロジックを実行するためのメソッド(execute)を一つもつ。  本ガイドラインでは、上記のようなインタフェースを、BLogicインタフェースと呼ぶ。 |
> 
> 定型的な共通処理をServiceに盛り込む場合、ビジネスロジックの処理フローを統一したい場合に、メソッドのシグネチャを限定するような基底クラスを作成することがある。
> 
> - シグネチャを限定するような基底クラス
> 	```java
> 	// (2)
> 	@Service
> 	@Transactional
> 	public abstract class AbstractBLogic<I, O> implements BLogic<I, O> {
> 	    public O execute(I input){
> 	      try{
> 	          // omitted
> 	          // (3)
> 	          preExecute(input);
> 	          // (4)
> 	          O output = doExecute(input);
> 	          // omitted
> 	          return output;
> 	      } finally {
> 	          // omitted
> 	      }
> 	    }
> 	    protected abstract void preExecute(I input);
> 	    protected abstract O doExecute(I input);
> 	}
> 	```
> 	| 項番 | 説明 |
> 	| --- | --- |
> 	| (2) | 基底クラスを作成する場合、 `@Transactional` の仕様上、AOPの対象となるのは外部から実行されるメソッドもしくはメソッドを実装しているクラスであるため、トランザクション制御が必要な場合はこの基底クラスに付与する。  `@Service` も同様に、 `ResultMessagesLoggingInterceptor` のようにAOPによってServiceを対象とするような場合はこの基底クラスに付与する必要がある。 |
> 	| (3) | 基底クラスより、業務ロジックを実行する前の、事前処理を行うメソッドを呼び出す。  上記のような事前処理を行うメソッドでは、ビジネスルールのチェックなどを実装することになる。 |
> 	| (4) | 基底クラスより、業務ロジックを実行するメソッドを呼び出す。 |
> 
> 以下に、シグネチャを限定するような、基底クラスを継承する場合の、サンプルを示す。
> 
> - BLogicクラス(Service)
> 	```java
> 	// (5)
> 	public interface XxxBLogic extends BLogic<XxxInput, XxxOutput> {
> 	}
> 	```
> 	| 項番 | 説明 |
> 	| --- | --- |
> 	| (5) | タイプセーフなインジェクションを可能にするために、BLogicインタフェースを継承したインタフェースを作成する。  親インタフェースのメソッド経由での呼び出しを行うために、BLogicを継承したサブインタフェースを実装する。 |
> 	```java
> 	@Service
> 	public class XxxBLogicImpl extends AbstractBLogic<XxxInput, XxxOutput> implements XxxBLogic {
> 	    // (6)
> 	    @Override
> 	    protected void preExecute(XxxInput input) {
> 	        // omitted
> 	        Tour tour = tourRepository.findById(input.getTourId());
> 	        Date reservationLimitDate = tour.reservationLimitDate();
> 	        if(input.getReservationDate().after(reservationLimitDate)){
> 	            throw new BusinessException(ResultMessages.error().add("e.xx.yy.0001"));
> 	        }
> 	    }
> 	    // (7)
> 	    @Override
> 	    protected XxxOutput doExecute(XxxInput input) {
> 	        TourReservation tourReservation = new TourReservation();
> 	        // omitted
> 	        tourReservationRepository.save(tourReservation);
> 	        XxxOutput output = new XxxOutput();
> 	        output.setTourReservation(tourReservation);
> 	        // omitted
> 	        return output;
> 	    }
> 	}
> 	```
> 	| 項番 | 説明 |
> 	| --- | --- |
> 	| (6) | 業務ロジックを実行する前の事前処理を実装する。  ビジネスルールのチェックなどを実装する事になる。 |
> 	| (7) | 業務ロジックを実装する。  ビジネスルールを充たすために、ロジックを実装する事になる。 |
> - Controller
> 	```java
> 	// (8)
> 	@Inject
> 	XxxBLogic xxxBLogic;
> 	public String reserve(XxxForm form, RedirectAttributes redirectAttributes) {
> 	    XxxInput input = new XxxInput();
> 	    // omitted
> 	    // (9)
> 	    XxxOutput output = xxxBlogic.execute(input);
> 	    // omitted
> 	    redirectAttributes.addFlashAttribute(output.getTourReservation());
> 	    return "redirect:/xxx?complete";
> 	}
> 	```
> 	| 項番 | 説明 |
> 	| --- | --- |
> 	| (8) | Controllerは、呼び出すBLogicインタフェースをInjectする。 |
> 	| (9) | Controllerは、BLogicインタフェースのexecuteメソッドを呼び出し、業務ロジックを実行する。 |

  

### ¶

Serviceの作成単位は主に以下の3パターンとなる。

> | 項番 | 単位 | 作成方法 | 特徴 |
> | --- | --- | --- | --- |
> |  | Entity毎 | 主体となるEntityと対でServiceを作成する。 | 主体となるEntityとは、業務データの事であり、 **業務データを中心にしてアプリケーションを設計・実装する場合は、この単位でServiceを作成することを推奨する。**      この単位でServiceを作成すると、業務データ毎に業務ロジックが集約されるため、業務処理の共通化が図られやすい。  ただし、このパターンでServiceを作成した場合、同時に大量の開発者を投入して作成するアプリケーションとの相性は、あまりよくない。どちらかと言うと、小規模・中規模のアプリケーションを開発する場合に向いているパターンと言える。 |
> |  | ユースケース毎 | ユースケースと対でServiceを作成する。 | **画面からのイベントを中心にしてアプリケーションを設計・実装する場合は、この単位でServiceを作成することになる。**      この単位でServiceを作成する場合は、ユースケース毎に担当者を割り当てることが出来るため、同時に大量の開発者を投入して開発するアプリケーションとの相性はよい。  一方で、このパターンでServiceを作成すると、ユースケース内での業務ロジックの共通化は行うことができるが、ユースケースを跨いだ業務ロジックの共通化は行われない可能性が高くなる。  ユースケースを跨いで業務ロジックの共通化を行う必要がある場合は、共通化を行うための共通チームを設けるなどの工夫が必要となる。 |
> | 3 | イベント毎 | 画面から発生するイベントと対でServiceを作成する。 | **画面からのイベントを中心にしてアプリケーションを設計・実装しBLogicクラスを生成する場合は、この単位でServiceを作成することになる。**  本ガイドラインでは、このような単位で作成されるServiceクラスの事を、BLogicと呼ぶ。      この単位でServiceを作成する場合の特徴としては、基本的にはユースケース毎に作成する際と同じである。  ただし、イベント毎にServiceクラスを設計・実装する事になるため、ユースケース毎に作成する場合に比べて、より共通化が行われない可能性が高くなる。  本ガイドラインとしては、イベント毎に作成するパターンは特に推奨しない。ただし、大規模開発において、保守性等を考慮して業務ロジックの作りを合わせておきたいといった理由がある場合は、イベント毎に作成する事を選択肢の一つとして考えてもよい。 |
> 
> Warning
> 
> **Serviceの作成単位については、開発するアプリケーションの特性や開発体制などを加味して決めて頂きたい。**
> 
> また、提示した３つの作成パターンの **どれか一つのパターンに絞る必要はない。** 無秩序にいろいろな単位のServiceを作成する事は避けるべきだが、 **アーキテクトによって方針が示されている状況下においては、併用しても特に問題はない。**
> 
> 例えば、以下のような組み合わせが考えられる。
> 
> 【組み合わせて使用する場合の例】
> 
> - アプリケーションとして重要な業務ロジックについては、Entity毎のSharedServiceクラスとして作成する。
> - 画面からのイベントを処理するための業務ロジックについては、Controller毎のServiceクラスとして作成する。
> - Controller毎のServiceクラスでは、必要に応じてSharedServiceクラスのメソッドを呼び出す事で業務ロジックを実装する。

  

### ¶

#### ¶

Serviceクラスを作成する際の注意点を、以下に示す。

- Serviceインタフェースの作成
	```java
	public interface CartService { // (1)
	    // omitted
	}
	```
	| 項番 | 説明 |
	| --- | --- |
	| (1) | **Serviceインタフェースを作成することを推奨する。**  インタフェースを設けることで、Serviceとして公開するメソッドを明確にすることが出来る。 |
	Note
	**アーキテクチャ観点でのメリット例**
	1. AOPを使う場合に、JDK標準のDynamic proxies機能が使われる。
		インタフェースがない場合はSpring Frameworkに内包されているCGLIBが使われるが、finalメソッドに対してAdviceできないなどの制約がある。
		詳細は、 [Spring Framework Documentation -Proxying Mechanisms-](https://docs.spring.io/spring-framework/docs/6.2.1/reference/html/core.html#aop-proxying) を参照されたい。
	2. 業務ロジックをスタブ化しやすくなる。
		アプリケーション層とドメイン層を別々の体制で並行して開発する場合は、アプリケーション層を開発するために、Serviceのスタブが必要になるケースがある。
		スタブを作成する必要がある場合は、インタフェースを設けておくことを推奨する。
- Serviceクラスの作成
	```java
	@Service // (2)
	@Transactional // (3)
	public class CartServiceImpl implements CartService { // (4) (5)
	    // omitted
	}
	```
	| 項番 | 説明 |
	| --- | --- |
	| (2) | **クラスに @Service アノテーションを付加する。**  アノテーションを付与することで、componentがscan対象となり、設定ファイルへのbean定義が、不要となる。 |
	| (3) | **クラスに @Transactional アノテーションを付加する。**  アノテーションを付与することで、すべての業務ロジックに対してトランザクション境界が設定される。  属性値については、要件に応じた値を指定すること。  詳細は、を参照されたい。      また、 `@Transactional` アノテーションを使用する際の注意点を理解するために、「」を合わせて確認するとよい。 |
	| (4) |  |
	| (5) | **Serviceクラスでは状態は保持せず、singletonスコープのbeanとしてコンテナに登録する 。**  フィールド変数には、スレッド毎に状態が変わるオブジェクト(Entity/DTO/VOなどのPOJO)や、値(プリミティブ型、プリミティブラッパークラスなど)を保持してはいけない。  また、 `@Scope` アノテーションを使ってsingleton以外のスコープ(prototype, request, session)にしてはいけない。 |
	Note
	**クラスに @Transactional アノテーションを付加する理由**
	トランザクション境界の設定が必須なのは更新処理を含む業務ロジックのみだが、設定漏れによるバグを防ぐ事を目的として、クラスレベルにアノテーションを付与することを推奨している。
	もちろん必要な箇所（更新処理を行うメソッド）のみに、 `@Transactional` アノテーションを定義する方法を採用してもよい。
	Note
	**singleton以外のスコープを禁止する理由**
	1. prototype, request, sessionは、状態を保持するbeanを登録するためのスコープであるため、Serviceクラスに対して使用すべきでない。
	2. スコープをrequestやprototypeにした場合、DIコンテナによるbeanの生成頻度が高くなるため、性能に影響を与えることがある。
	3. スコープをrequestやsessionにした場合、Webアプリケーション以外のアプリケーション(例えば、Batchアプリケーションなど)で使用できなくなる。

  

#### ¶

Serviceクラスのメソッドを作成する際の注意点を、以下に示す。

- Serviceインタフェースのメソッド作成
	```java
	public interface CartService {
	    Cart createCart(); // (1) (2)
	    Cart findCart(String cartId); // (1) (2)
	}
	```
- Serviceクラスのメソッドの作成
	```java
	@Service
	@Transactional
	public class CartServiceImpl implements CartService {
	    @Inject
	    CartRepository cartRepository;
	    public Cart createCart() { // (1) (2)
	        Cart cart = new Cart();
	        // omitted
	        cartRepository.save(cart);
	        return cart;
	    }
	    @Transactional(readOnly = true) // (3)
	    public Cart findCart(String cartId) { // (1) (2)
	        Cart cart = cartRepository.findByCartId(cartId);
	        // omitted
	        return cart;
	    }
	}
	```
	| 項番 | 説明 |
	| --- | --- |
	| (1) | **Serviceクラスのメソッドは、業務ロジック毎に作成する。** |
	| (2) | **業務ロジックは、Serviceインタフェースでメソッドの定義を行い、Serviceクラスのメソッドで実装を行う。** |
	| (3) | **業務ロジックのトランザクション定義をデフォルト（クラスアノテーションで指定した定義）から変更する場合は、@Transactionalアノテーションを付加する。**  属性値については、要件に応じた値を指定すること。  詳細は、を参照されたい。      また、 `@Transactional` アノテーションを使用する際の注意点を理解するために、「」を合わせて確認するとよい。 |
	Tip
	**参照系の業務ロジックのトランザクション定義について**
	参照系の業務ロジックを実装する場合は、 `@Transactional(readOnly = true)` を指定することで、JDBCドライバに対して「読み取り専用のトランザクション」のもとでSQLを実行するように指示することができる。
	読み取り専用のトランザクションの扱い方は、JDBCドライバの実装に依存するため、使用するJDBCドライバの仕様を確認されたい。
	Note
	**新しいトランザクションを開始する必要がある場合のトランザクション定義について**
	呼び出し元のメソッドが参加しているトランザクションには参加せず、新しいトランザクションを開始する必要がある場合は、 `@Transactional(propagation = Propagation.REQUIRES_NEW)` を設定する。

  

#### ¶

Serviceクラスのメソッド引数と返り値は、以下の点を考慮すること。

Serviceクラスの引数と返り値は、Serialize可能なクラス(`java.io.Serializable` を実装しているクラス)とする。

Serviceクラスは、分散アプリケーションとしてデプロイされる可能性もあるので、引数と返り値は、Serialize可能なクラスのみ、許可することを推奨する。

**メソッド引数/返り値となる代表的な型を以下に示す。**

- プリミティブ型(`int`, `long` など)
- プリミティブラッパークラス(`java.lang.Integer`, `java.lang.Long` など)
- java標準クラス(`java.lang.String`, `java.util.Date` など)
- ドメインオブジェクト(Entity、DTOなど)
- 入出力オブジェクト(DTO)
- 上記型のコレクション(`java.util.Collection` の実装クラス)
- void
- etc …
	Note
	**入出力オブジェクトとは**
	1. 入力オブジェクトとは、Serviceのメソッドを実行するために必要な入力値をまとめたオブジェクトのことをさす。
	2. 出力オブジェクトとは、Serviceのメソッドの実行結果（出力値）をまとめたオブジェクトのことをさす。

**メソッド引数/返り値として禁止するものを以下に示す。**

- アプリケーション層の実装アーキテクチャ(Servlet APIやSpringのweb層のAPIなど)に依存するオブジェクト(`jakarta.servlet.http.HttpServletRequest` 、 `jakarta.servlet.http.HttpServletResponse` 、 `jakarta.servlet.http.HttpSession` 、 `org.springframework.http.server.ServletServerHttpRequest` など)
- アプリケーション層のモデル(Form,DTOなど)
- `java.util.Map` の実装クラス
	Note
	**禁止する理由**
	1. アプリケーション層の実装アーキテクチャに依存するオブジェクトを許可してしまうと、アプリケーション層とドメイン層が密結合になってしまう。
	2. `java.util.Map` は、インタフェースとして汎用性が高すぎるため、メソッドの引数や返り値に使うとどのようなオブジェクトが格納されているかわかりづらい。
		また、値の管理がキー名で行われるため、以下の問題が発生しやすくなる。
		- 値を設定する処理と値を取得する処理で異なるキー名を指定してしまい、値が取得できない。
		- キー名の変更した場合の影響範囲の把握が困難になる。

**アプリケーション層とドメイン層で同じDTOを共有する場合の方針を、以下に示す。**

- ドメイン層のパッケージに属するDTOとして作成し、アプリケーション層で利用する。
	Warning
	アプリケーション層のFormやDTOを、ドメイン層で利用してはいけない。

  

### ¶

ServiceおよびSharedServiceのメソッドで実装する処理について説明する。

ServiceおよびSharedServiceでは、アプリケーションで使用する業務データの取得、更新、整合性チェックおよびビジネスルールに関わる各種ロジックの実装を行う。

以下に、代表的な処理の実装例について説明する。

  

#### ¶

警告メッセージの返却は、戻り値としてメッセージオブジェクトを返却する。

Entityなどのドメイン層のオブジェクトと一緒に返却する必要がある場合は、出力オブジェクト(DTO)にメッセージオブジェクトとドメインオブジェクトを詰めて返却する。

共通ライブラリとしてメッセージオブジェクト(`org.terasoluna.gfw.common.message.ResultMessages`)を用意している。

共通ライブラリで用意しているクラスだと要件を満たせない場合は、プロジェクト毎にメッセージオブジェクトを作成すること。

- DTOの作成
	```java
	public class OrderResult implements Serializable {
	    private ResultMessages warnMessages;
	    private Order order;
	    // omitted
	}
	```

  

- Serviceクラスのメソッドの実装
	下記の例では、注文した商品の中に取り寄せ商品が含まれているため、分割配達となる可能性がある旨を警告メッセージとして表示する場合の実装例である。
	```java
	public OrderResult submitOrder(Order order) {
	    // omitted
	    boolean hasOrderProduct = orderRepository.existsByOrderProduct(order); // (1)
	    // omitted
	    Order order = orderRepository.save(order);
	    // omitted
	    ResultMessages warnMessages = null;
	    // (2)
	    if(hasOrderProduct) {
	        warnMessages = ResultMessages.warning().add("w.xx.yy.0001");
	    }
	    // (3)
	    OrderResult orderResult = new OrderResult();
	    orderResult.setOrder(order);
	    orderResult.setWarnMessages(warnMessages);
	    return orderResult;
	}
	```
	| 項番 | 説明 |
	| --- | --- |
	| (1) | 取り寄せ商品が含まれる場合は、 `hasOrderProduct` に `true` が設定される。 |
	| (2) | 上記例では、取り寄せ商品が含まれる場合に、警告メッセージを生成している。 |
	| (3) | 上記例では、登録した `Order` オブジェクトと警告メッセージを一緒に返却するために、 `OrderResult` というDTOにオブジェクトを格納して返却している。 |

  

#### ¶

業務ロジック実行中に、ビジネスルールの違反が発生した場合はビジネス例外をスローする。

例えば次のような場合である。

- 旅行を予約する際に予約日が期限を過ぎている場合
- 商品を注文する際に在庫切れの場合
- etc …

共通ライブラリとしてビジネス例外(`org.terasoluna.gfw.common.exception.BusinessException`)を用意している。

共通ライブラリで用意しているビジネス例外クラスだと要件を満たせない場合は、プロジェクト毎にビジネス例外クラスを作成すること。

**ビジネス例外クラスは、java.lang.RuntimeException のサブクラスとして作成することを推奨する** 。

> Note
> 
> **ビジネス例外を非検査例外にする理由**
> 
> ビジネス例外は、Controllerでハンドリングが必要になるため、本来は検査例外にした方がよい。しかし、本ガイドラインでは、設定漏れによるバグを防ぐ事を目的として、デフォルトでロールバックされる java.lang.RuntimeException のサブクラスとすることを推奨する。もちろん検査例外のサブクラスとしてビジネス例外を作成し、ビジネス例外クラスをロールバック対象として定義する方法を採用してもよい。

ビジネス例外のスロー例を以下に示す。

下記の例では、予約期限日が過ぎていることを業務エラーとして通知する際の実装例である。

> ```java
> // omitted
> 
> if(currentDate.after(reservationLimitDate)) { // (1)
>     throw new BusinessException(ResultMessages.error().add("e.xx.yy.0001"));
> }
> 
> // omitted
> ```
> 
> | 項番 | 説明 |
> | --- | --- |
> | (1) | 旅行を予約する際に、予約日が期限を過ぎているので、ビジネス例外をスローしている。 |

例外ハンドリング全体の詳細は、 [例外ハンドリング](https://terasolunaorg.github.io/guideline/current/ja/ArchitectureInDetail/WebApplicationDetail/ExceptionHandling.html) を参照されたい。

  

#### ¶

業務ロジック実行中に、システムとして異常な状態が発生した場合は、システム例外をスローする。

例えば、次のような場合である。

- 事前に存在しているはずのマスタデータ、ディレクトリ、ファイルなどが存在しない場合
- 利用しているライブラリのメソッドから発生する検査例外のうち、システム異常に分類される例外を補足した場合
- etc …

共通ライブラリとしてシステム例外(`org.terasoluna.gfw.common.exception.SystemException`)を用意している。

共通ライブラリで用意しているシステム例外クラスだと要件を満たせない場合は、プロジェクト毎にシステム例外クラスを作成すること。

**システム例外クラスは、java.lang.RuntimeException のサブクラスとして作成することを推奨する** 。

理由は、システム例外は、アプリケーションのコード上でハンドリングする必要がないという点と、 `@Transactinal` アノテーションのデフォルトのロールバック対象が、 `java.lang.RuntimeException` のためである。

システム例外のスロー例を以下に示す。

下記の例では、指定された商品が、商品マスタに存在しないことを、システムエラーとして通知する際の実装例である。

> ```java
> ItemMaster itemMaster = itemMasterRepository.findById(itemCode);
> if(itemMaster == null) { // (1)
>     throw new SystemException("e.xx.fw.0001",
>         "Item master data is not found. item code is " + itemCode + ".");
> }
> ```
> 
> | 項番 | 説明 |
> | --- | --- |
> | (1) | 事前に存在しているはずのマスタデータがないので、システム例外をスローしている。（ロジックで、システム異常を検知した場合の実装例） |

下記の例では、ファイルコピー時のIOエラーをシステムエラーとして通知する際の実装例である。

> ```java
> // omitted
> 
> try {
>     FileUtils.copy(srcFile, destFile);
> } catch(IOException e) { // (1)
>     throw new SystemException("e.xx.fw.0002",
>         "Failed file copy. src file '" + srcFile + "' dest file '" + destFile + "'.", e);
> }
> ```
> 
> | 項番 | 説明 |
> | --- | --- |
> | (1) | 利用しているライブラリのメソッドから、システム異常に分類される例外が発生したシステム例外をスローしている。  **利用しているライブラリから発生した例外は、原因例外としてシステム例外クラスに必ず渡すこと。**  原因例外が失われると、スタックトレースよりエラー発生箇所および本質的なエラー原因が追えなくなってしまう。 |
> 
> Note
> 
> **データアクセスエラーの扱いについて**
> 
> 業務ロジック実行中に、RepositoryやO/R Mapperでデータアクセスエラーが発生した場合、 `org.springframework.dao.DataAccessException` のサブクラスに変換されてスローされる。
> 
> 基本的には、業務ロジックではキャッチせず、アプリケーション層でエラーハンドリングすればよいが、一意制約違反などの一部のエラーについては、業務要件によっては、業務ロジックでハンドリングする必要がある。
> 
> 詳細は、 [データベースアクセス（共通編）](https://terasolunaorg.github.io/guideline/current/ja/ArchitectureInDetail/DataAccessDetail/DataAccessCommon.html) を参照されたい。

  

## ¶

データの一貫性を保証する必要がある処理ではトランザクションの管理が必要となる。

  

### ¶

トランザクションの管理方法はいろいろあるが、本ガイドラインでは、 **Spring Frameworkから提供されている「宣言型トランザクション管理」を利用することを推奨する。**

  

#### ¶

「宣言型トランザクション管理」では、トランザクション管理に必要な情報を以下に２つの方法で宣言することができる。

- XML(bean定義ファイル)で宣言する。
- **アノテーション（@Transactional）で宣言する。（推奨）**

Spring Frameworkから提供されている「宣言型トランザクション管理」の詳細については、 [Spring Framework Documentation -Declarative transaction management-](https://docs.spring.io/spring-framework/docs/6.2.1/reference/html/data-access.html#transaction-declarative) を参照されたい。

> Note
> 
> **「アノテーションで指定する」方法を推奨する理由**
> 
> 1. ソースコードを見ただけで、どのようなトランザクション管理が行われるかについて、把握することができる。
> 2. XMLにトランザクション管理するためのAOPの設定が不要であり、XMLがシンプルになる。

  

#### ¶

トランザクション管理対象とするクラスまたはクラスメソッドに対して `@Transactional` アノテーションを指定する。

トランザクション制御に必要となる情報は、 `@Transactional` アノテーションの属性で指定する。

> Note
> 
> 本ガイドラインでは、Spring Frameworkから提供されている `@org.springframework.transaction.annotation.Transactional` アノテーションを使用する前提である。
> 
> Tip
> 
> Spring 4からは、JTA 1.2から追加された `@jakarta.transaction.Transactional` アノテーションを使用する事ができる。
> 
> ただし、本ガイドラインでは、「宣言型トランザクション管理」で必要となる情報をより細かく指定できるSpring Frameworkのアノテーションを使用することを推奨する。
> 
> Spring Frameworkのアノテーションを使用すると、
> 
> - トランザクションの伝播方法(`propagation` 属性)の属性値として `NESTED` (JDBCのセーブポイント)
> - トランザクションの独立レベル(`isolation` 属性)
> - トランザクションのタイムアウト時間(`timeout` 属性)
> - トランザクションの読み取り専用フラグ(`readOnly` 属性)
> 
> の指定が可能となる。
> 
> | 項番 | 属性名 | 説明 |
> | --- | --- | --- |
> | 1 | propagation | トランザクションの伝播方法を指定する。      **\[REQUIRED\]**  トランザクションが開始されていなければ開始する。 (省略時のデフォルト)  **\[REQUIRES\_NEW\]**  常に、新しいトランザクションを開始する。  **\[SUPPORTS\]**  トランザクションが開始されていれば、それを利用する。開始されていなければ、利用しない。  **\[NOT\_SUPPORTED\]**  トランザクションを利用しない。  **\[MANDATORY\]**  トランザクションが開始されている必要がある。開始されていなければ、例外が発生する。  **\[NEVER\]**  トランザクションを利用しない（開始されていてはいけない）。開始していれば、例外が発生する。  **\[NESTED\]**  セーブポイントが設定される。JDBCのみ有効である。 |
> | 2 | isolation | トランザクションの独立レベルを指定する。  この設定は、DBの仕様に依存するため、使用するDBの仕様を確認し、設定値を決めること。      **\[DEFAULT\]**  DBが提供するデフォルトの独立性レベル。(省略時のデフォルト)  **\[READ\_UNCOMMITTED\]**  他のトランザクションで変更中（未コミット）のデータが読める。  **\[READ\_COMMITTED\]**  他のトランザクションで変更中（未コミット）のデータは読めない。  **\[REPEATABLE\_READ\]**  他のトランザクションが読み出したデータは更新できない。  **\[SERIALIZABLE\]**  トランザクションを完全に独立させる。      トランザクションの独立レベルは、排他制御に関連するパラメータとなる。  排他制御については、 [排他制御](https://terasolunaorg.github.io/guideline/current/ja/ArchitectureInDetail/DataAccessDetail/ExclusionControl.html) を参照されたい。 |
> | 3 | timeout | トランザクションのタイムアウト時間(秒)を指定する。  デフォルトは-1(使用するDBの仕様や設定に依存) |
> | 4 | readOnly | トランザクションの読み取り専用フラグを指定する。  デフォルトはfalse(読み取り専用でない) |
> | 5 | rollbackFor | トランザクションのロールバック対象とする例外クラスのリストを指定する。  デフォルトは空（指定なし） |
> | 6 | rollbackForClassName | トランザクションのロールバック対象とする例外クラス名のリストを指定する。  デフォルトは空（指定なし） |
> | 7 | noRollbackFor | トランザクションのコミット対象とする例外クラスのリストを指定する。  デフォルトは空（指定なし） |
> | 8 | noRollbackForClassName | トランザクションのコミット対象とする例外クラス名のリストを指定する。  デフォルトは空（指定なし） |
> 
> Note
> 
> **@Transactionalアノテーションを指定する場所**
> 
> **クラスまたはクラスのメソッドに指定することを推奨する。**
> 
> インタフェースまたはインタフェースのメソッドを指定しない理由については、 [Spring Framework Documentation -Using @Transactional-](https://docs.spring.io/spring-framework/docs/6.2.1/reference/html/data-access.html#transaction-declarative-annotations) の2個目のTipsを参照されたい。
> 
> Warning
> 
> **例外発生時のrollbackとcommitのデフォルト動作**
> 
> rollbackForおよびnoRollbackForを指定しない場合、Spring Frameworkは以下の動作となる。
> 
> - 非検査例外クラス（java.lang.RuntimeExceptionおよびjava.lang.Error）またはそのサブクラスの例外が発生した場合は、rollbackする。
> - 検査例外クラス（java.lang.Exception）またはそのサブクラスの例外が発生した場合は、commitする。 **(注意が必要)**
> 
> Note
> 
> **@Transactionalアノテーションのvalue属性について**
> 
> `@Transactional` アノテーションにはvalue属性があるが、これは複数のTransaction Managerを宣言した際に、どのTransaction Managerを使うのかを指定する属性である。Transaction Managerが一つの場合、指定は不要である。
> 
> 複数のTransaction Managerを使う必要がある場合は、 [Spring Framework Documentation -Multiple Transaction Managers with @Transactional-](https://docs.spring.io/spring-framework/docs/6.2.1/reference/html/data-access.html#tx-multiple-tx-mgrs-with-attransactional) を参照されたい。
> 
> Note
> 
> **主要DBのisolationのデフォルトについて**
> 
> 主要DBのデフォルトの独立性レベルは、以下の通りである。
> 
> - Oracle: READ\_COMMITTED
> - DB2: READ\_COMMITTED
> - PostgreSQL: READ\_COMMITTED
> - SQL Server: READ\_COMMITTED
> - MySQL: REPEATABLE\_READ
> 
> Note
> 
> **@Transactionalアノテーションのtimeout属性について**
> 
> クエリ発行時（Repositoryのメソッド実行時）に `timeout` 属性に指定した時間に従って、トランザクションタイムアウトのチェックが行なわれるが、このときの挙動について以下の点に注意されたい。
> 
> - タイムアウトチェック時に既にタイムアウトしていないかを確認するため、 `timeout` 属性に指定した時間が経過したタイミングで例外が発生するわけではない。
> - タイムアウトチェック後に、関係ない業務処理にいくら時間がかかってもタイムアウトにはならない。
> 
> また、トランザクションタイムアウトに関して以下の事象にも注意されたい。
> 
> - クエリを発行した後のタイムアウトの挙動はJDBCドライバの実装に依存する。
> - 使用するTransaction Managerによっては、コミット時にもトランザクションタイムアウトのチェックが行われる。

  

#### ¶

トランザクションの伝播方法は、ほとんどの場合は「REQUIRED」でよい。

ただし、 **アプリケーションの要件によっては「REQUIRES\_NEW」を使うこともある** ので、「REQUIRED」と「REQUIRES\_NEW」を指定した場合のトランザクション制御フローを、以下に示す。

他の伝播方法の使用頻度は低いと思われるので、本ガイドラインでの説明は省略する。

**トランザクションの伝播方法を「REQUIRED」にした場合のトランザクション管理フロー**

トランザクションの伝播方法を「REQUIRED」にした場合、Controllerから呼び出された一連の処理が、すべて同じトランザクション内で処理される。

> ![transaction management flow of REQUIRED](https://terasolunaorg.github.io/guideline/current/ja/_images/service_transaction-propagation-required.png)
> 
> transaction management flow of REQUIRED

1. Controllerからトランザクション管理対象のServiceのメソッドを呼び出す。
	この時点で開始されているトランザクションは存在しないため、 `TransactionInterceptor` によってトランザクションが開始される。
2. `TransactionInterceptor` は、トランザクション開始した後に、トランザクション管理対象のメソッドを呼び出す。
3. `TransactionInterceptor` は、開始済みのトランザクションに参加した後に、トランザクション管理対象のメソッドを呼び出す。
4. `TransactionInterceptor` は、処理結果に応じてコミットまたはロールバックを行い、トランザクションを終了する。

> ![transaction management flow of REQUIRES_NEW](https://terasolunaorg.github.io/guideline/current/ja/_images/service_transaction-propagation-requires_new.png)
> 
> transaction management flow of REQUIRES\_NEW

1. Controllerからトランザクション管理対象のServiceのメソッドを呼び出す。この時点で開始されているトランザクションは存在しないため、 `TransactionInterceptor` によってトランザクションが開始される(ここで開始したトランザクションを以降「Transaction A」と呼ぶ)。
2. `TransactionInterceptor` は、トランザクション（Transaction A）を開始した後に、トランザクション管理対象のメソッドを呼び出す。
3. `TransactionInterceptor` は、トランザクション（Transaction B）を開始した後に、トランザクション管理対象のメソッドを呼び出す。
4. `TransactionInterceptor` は、処理結果に応じてコミットまたはロールバックを行い、トランザクション（Transaction B）を終了する。
	この時点で、「Transaction A」のトランザクションが再開され、アクティブな状態になる。
5. `TransactionInterceptor` は、処理結果に応じてコミットまたはロールバックを行い、トランザクション（Transaction A）を終了する。
