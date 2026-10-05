# Ore 語言設計文件（草案）

Oct 5, 2026 · @Solemnis

## 目標與非目標

Ore（暫定名）是一門沒有 GC、以 explicit allocator 為核心的系統語言，同時服務兩個用途：通用系統程式（CLI、server、工具），以及 anyknown browser 自家引擎的實作語言。第二個用途是設計的壓力測試：引擎需要的東西（可預測延遲、圖狀資料結構、大量 C 函式庫、多執行緒）決定了大部分取捨。

**目標**

- **可預測的效能**：沒有隱藏配置、沒有隱藏控制流，讀程式碼就知道哪裡會配置記憶體、哪裡會跳轉。
- **零成本 C 互通**：直接呼叫 C ABI，不需要 wrapper 或 marshalling。引擎會大量依賴既有 C/C++ 函式庫。
- **預設安全檢查**：debug 與 safe-release 模式下做 bounds、overflow、null、use-after-free 偵測；只有 fast-release 關掉。
- **編譯快**：debug build 的迭代速度優先於 release 的極致最佳化。
- **單一工具鏈**：`ore build / run / test / fmt / doc`，跟 Go 一樣不需要外部建置工具。
- **AI 協作優先**：AI agent 寫的 Ore 要能一次寫對，寫錯時靠編譯器回饋就能修好。見「AI 協作」一節。

**非目標**

- 不做 GC，也不做 Rust 式 borrow checker。
- 不做 exception、繼承、文字巨集。
- 不取代 JavaScript：網頁內容的腳本語言仍是 JS，Ore 只寫引擎本身。
- 1.0 之前不保證語法與 ABI 穩定。

## 設計原則與定位

Ore 的定位是「Zig 的記憶體模型 + Go 的簡單與工具鏈 + Rust 的 tagged union」，其餘功能一律先不加。

1. **沒有隱藏配置**：會配置記憶體的函式一定接收 `Allocator` 參數。
2. **沒有隱藏控制流**：沒有 exception、沒有一般性的 operator overloading、沒有自動執行的 destructor。清理靠我們看得到的 `defer`。
3. **編譯期執行取代巨集**：泛型與程式碼產生都用 `comptime` 寫一般的 Ore，不另外發明巨集語言。
4. **C 是一等公民**：C ABI、C header、C 工具鏈（clang 的最佳化與 sanitizer）都直接用。
5. **一件事一種寫法**：語法小到能在一個週末學完，格式由 `ore fmt` 決定，沒有討論空間。
6. **為 AI 寫碼設計**：新語言沒有訓練資料，只能靠三件事補：語法像 AI 已經熟悉的語言、整份規格放得進一個 context、編譯器回饋精確到可以照著改。語法取捨有疑義時，選 AI 最不容易寫錯的寫法。

| 語言 | 記憶體 | 泛型 | 錯誤處理 | Ore 借用 | Ore 不採用 |
| --- | --- | --- | --- | --- | --- |
| Go | GC | type parameters | 多回傳值 `error` | 簡單語法、單一工具鏈、目錄即套件、`defer` | GC、goroutine runtime、interface 隱式 boxing |
| Zig | 手動 + allocator 參數 | `comptime` | error union | allocator 參數、`comptime`、`errdefer`、C 互通、三種 build mode | `anytype` duck typing（錯誤訊息難讀） |
| Rust | ownership + borrow | trait + monomorphization | `Result` | 帶 payload 的 enum、窮舉 `match`、預設不可變 | borrow checker、程序巨集、複雜的 trait 系統 |
| Odin | 手動 + 隱式 context allocator | parametric polymorphism | 多回傳值 | 內建 SIMD vector 型別 | 隱式 `context`（違反「沒有隱藏配置」） |

## 記憶體模型

**決策：手動管理 + explicit `Allocator` + `defer`，再加上兩個一等公民：arena 與 generational handle。** 安全性靠執行期檢查與 debug allocator，不靠編譯期證明。

**為什麼不用 GC**

- 引擎每一幀只有約 16.6 ms（60 Hz），GC 暫停會直接變成掉幀。
- JS engine 自己已經有 GC。引擎語言再帶一個 GC，兩個 heap 之間的循環參照會非常難處理。
- 實作成本：GC + runtime 是整個編譯器以外最大的工程。

**為什麼不用 borrow checker**

- DOM、layout tree、render tree 都是帶 parent / sibling 指標的循環圖，borrow checker 正好最不擅長這種結構。
- borrow checker 與 lifetime 推導是型別系統裡最難做對的部分，會讓語言本身變成主要工程。

**四個機制**

1. `Allocator` 是一個小介面（`alloc` / `resize` / `free`），由呼叫端傳入。std 提供 `General`（debug 模式偵測 leak、double free、use-after-free）、`Arena`、`Pool(T)`、`FixedBuffer`。
2. `defer` / `errdefer`：離開 scope 時執行；`errdefer` 只在以錯誤離開時執行。
3. **Arena**：生命週期一致的一批資料一次釋放，整個類別的 use-after-free 與 leak 直接消失。
4. **`Handle(T)`**：`u32` index + `u32` generation，指向 `Pool(T)` 的 slot。slot 釋放後 generation 加一，舊 handle 解參照時回傳 `null`，不會讀到垃圾。長生命週期的圖結構一律用 handle，不用裸指標。

**引擎的配置策略**

| 資料 | 生命週期 | 配置方式 |
| --- | --- | --- |
| style 計算、layout、paint 的暫存資料 | 一幀 | per-frame `Arena`，幀結束時 reset |
| 解析後的 stylesheet、字串池 | 一個 document | per-document `Arena` |
| DOM node、layout box | 不固定，可能被 JS 持有 | `Pool(T)` + `Handle(T)` |
| 網路 buffer、圖片解碼結果 | 不固定 | `General`，配合 `defer` |

**代價**：fast-release 模式下，裸指標仍可能 use-after-free。緩解方式是前述的 handle 規則、debug allocator，以及 CI 跑 ASan（C backend 直接獲得）。

## 型別系統

靜態、名稱型別、沒有隱式轉型；泛型靠 `comptime` 參數加上 `where` 約束，執行期多型靠明確的 `dyn` 胖指標。

| 類別 | 語法 | 說明 |
| --- | --- | --- |
| 整數 | `i8`…`i64`、`u8`…`u64`、`isize`、`usize` | 溢位在 debug / safe 模式 trap；要環繞就用 `+%`、`*%` |
| 浮點、布林 | `f32`、`f64`、`bool` | 數值轉型一律寫 `x as f32` |
| SIMD | `simd[4]f32` | 內建算術運算子，給圖形與 layout 用；是唯一有運算子的非純量型別 |
| 指標 | `*T`、`*mut T` | 非 null；預設唯讀，可寫要明寫 `mut` |
| 可空值 | `?T` | `?*T` 與裸指標一樣大，null 只能透過 `?` 表達 |
| 陣列、slice | `[N]T`、`[]T` | slice = 指標 + 長度，索引會做 bounds check |
| C 指標 | `[*]T` | 只給 FFI 用，沒有長度、沒有檢查 |
| 字串 | `[]u8` | 慣例上是 UTF-8，沒有獨立的 string 型別 |
| struct | `struct { x: f32 }` | 編譯器可重排欄位；`extern struct` 保證 C 配置 |
| enum | `enum Shape { Circle { r: f32 }, Empty }` | 可帶 payload 的 tagged union，`match` 必須窮舉 |
| 錯誤聯合 | `!T`、`E!T` | 見下一節 |
| 介面 | `interface Writer { fn write(...) }` | 用在 `where` 約束（編譯期）或 `dyn Writer`（執行期 vtable） |

**泛型**：型別是 `comptime` 值，泛型函式就是接收 `comptime T: type` 的一般函式，泛型型別就是回傳 `type` 的函式。與 Zig 不同的是，參數可以寫 `where T: Hash`，實例化失敗時錯誤指向約束，不是泛型函式內部的某一行。

**`dyn`**：`dyn Writer` = 資料指標 + vtable 指標，指向已存在的值，不會配置記憶體。這是跟 Go interface 最大的差異。

## 錯誤處理

可恢復的錯誤是回傳值（error union），程式 bug 是 `panic`（直接中止），沒有第三種。

- **Error set**：`error ParseError { UnexpectedEof, InvalidByte }`。兩個 set 可用 `||` 合併；回傳型別寫 `!T` 時，由編譯器推導出完整的 set。
- **`try expr`**：錯誤就直接往上回傳，等同 Rust 的 `?`。
- **`expr catch |e| { ... }`**：就地處理。`catch` 後的 `match e` 必須窮舉，新增錯誤種類時編譯器會指出所有漏接的地方。
- **`errdefer`**：只在以錯誤離開 scope 時執行，處理「做到一半失敗」的清理。
- **Error return trace**：debug 模式下記錄錯誤一路經過的 `try`，不需要 stack unwinding。

錯誤值本身不帶 payload，只是一個整數。需要細節的場景（例如 HTML、CSS parser 的錯誤位置）以 `*mut Diagnostics` 參數傳出，錯誤路徑上就不會有隱藏配置。注意：引擎的 HTML parser 依規格永遠不會失敗，錯誤只是診斷訊息。

## 語法概覽

語法接近 Go：大括號、型別寫在名稱後面、不寫分號（採用 Go 的自動插入規則）。`let` 宣告不可變綁定，`var` 宣告可變綁定；`pub` 導出。

**範例一：allocator、`defer`、錯誤處理**

```ore
import std
import std.mem.Allocator

fn readConfig(alloc: Allocator, path: []u8) !Config {
    let file = try std.fs.open(path)
    defer file.close()

    let bytes = try file.readAll(alloc)
    defer alloc.free(bytes)

    var cfg = try Config.parse(alloc, bytes)
    errdefer cfg.deinit(alloc)      // validate 失敗才會釋放

    try cfg.validate()
    return cfg
}

pub fn main() !void {
    var gpa = std.heap.General.init()
    defer gpa.deinit()              // debug 模式下回報 leak

    let cfg = readConfig(gpa.allocator(), "app.toml") catch |e| {
        std.log.err("config: {}", e)
        return e
    }
    defer cfg.deinit(gpa.allocator())
}
```

**範例二：tagged union 與窮舉 `match`**

```ore
enum Shape {
    Circle { center: Point, r: f32 }
    Rect { min: Point, max: Point }
}

fn area(s: Shape) f32 {
    return match s {
        .Circle { r, .. } => std.math.pi * r * r
        .Rect { min, max } => (max.x - min.x) * (max.y - min.y)
    }
}
```

**範例三：`comptime` 泛型與 handle**

```ore
pub fn Pool(comptime T: type) type {
    return struct {
        slots: []Slot(T)
        free_list: ?u32

        pub fn get(self: *Self, h: Handle(T)) ?*mut T {
            let slot = &mut self.slots[h.index]
            if slot.generation != h.generation { return null }
            return &mut slot.value
        }
    }
}

// 使用端：節點已被移除時拿到 null，不會讀到垃圾
if nodes.get(target) |node| {
    node.dirty = true
}
```

第一版的完整關鍵字清單（共 28 個：下列 26 個，加上型別修飾用的 mut 與 extern）：`fn` `let` `var` `pub` `import` `struct` `enum` `error` `interface` `dyn` `match` `if` `else` `for` `while` `break` `continue` `return` `defer` `errdefer` `try` `catch` `comptime` `where` `as` `test`。

## 模組、套件與建置

一個目錄是一個模組（Go 的規則），建置腳本用 Ore 寫（Zig 的規則），相依套件用內容雜湊鎖定。

- **模組**：同一目錄的 `.ore` 檔共用一個命名空間，只有 `pub` 的東西會導出。沒有 header 檔，模組之間不允許循環 import。
- **建置腳本**：`build.ore` 在編譯期直譯執行，宣告 target、相依套件、要連結的 C 函式庫。不需要 Make、CMake。
- **相依套件**：`build.ore` 寫來源 URL，`ore.lock` 記錄內容雜湊。沒有中央 registry，第一版也不做版本解析。
- **測試**：`test "名稱" { ... }` 直接寫在原始碼旁邊，`ore test` 執行。測試預設用 debug allocator，leak 就是測試失敗。
- **Build mode**：`debug`（全部檢查、不最佳化）、`safe`（最佳化 + 檢查）、`fast`（最佳化、關掉檢查）。也可以用 `@setSafety(off)` 對單一熱路徑關掉檢查。
- **交叉編譯**：`ore build --target aarch64-macos`。第一階段由 clang 處理 target，見編譯器架構一節。

## AI 協作

**決策：「AI agent 能一次寫對，寫錯時能靠編譯器回饋修好」是語言與工具鏈的驗收標準之一。** AI 對 Ore 的認識只會有三個來源：Ore 跟既有語言有多像、工具鏈附帶的規格、編譯器與測試的回饋。這三個來源都要刻意設計。

**語言層規則**

| 規則 | 對 AI 的理由 |
| --- | --- |
| 局部可推理：只看函式簽章與本體，就知道型別、會不會配置記憶體、會不會失敗 | AI 一次只看得到部分程式碼，需要跨檔推導的語意最容易寫錯 |
| `pub` 函式必須寫出錯誤集；推導式 `!T` 只能用在模組內部 | 推導出來的錯誤集要看完整條呼叫鏈才知道，AI 寫窮舉 `catch` 時無從得知要接哪些錯誤 |
| 任何 scope 都不允許遮蔽（shadowing） | 同名綁定是 AI 修改程式碼時最常接錯的地方 |
| 沒有函式多載、預設參數、隱式轉型，也沒有一般性的 operator overloading | 一個名字只對應一個定義 |
| 宣告一律以「關鍵字 + 名稱」開頭（`fn foo`、`struct Foo`、`enum Foo`、`error Foo`） | 用 `rg "fn foo"` 就能找到定義，不依賴 LSP |
| import 不支援 glob；每個外部名稱都能追到某一行 import | 名稱來源唯一、可查 |
| 語法有疑義時，選 Go / Rust / Zig 裡最常見而且語意相同的寫法；不允許「寫法相同、意思不同」 | AI 的直覺來自這些語言。寫法相同但意思不同的語法，比沒見過的語法更容易出錯 |
| `comptime` 只做純計算（不讀檔、不讀環境變數）；泛型參數若用到「搬移、取大小」以外的操作，必須寫 `where` 約束 | AI 無法在腦中執行 comptime，約束讓錯誤出現在呼叫端，而不是泛型內部 |

**工具鏈層規則**

- **規格隨工具鏈發布**：`ore spec` 輸出單一文件，內容是語法與型別規則，控制在約 20k tokens 以內，版本跟編譯器綁定。`ore doc --signatures <module>` 輸出 std 某個模組的公開簽章。AI agent 開始工作前，先把這兩份放進 context。
- **機器可讀的診斷**：`ore check --json` 為每個錯誤輸出固定的錯誤代碼（例如 `E0312`）、位置範圍、訊息，以及可以直接套用的修正（fix-it：檔案、範圍、替換文字）。`ore explain E0312` 給出錯誤範例與正確寫法。
- **一次回報所有錯誤**：parser 與 sema 要能從錯誤中恢復，把錯誤全部回報，而不是只報第一個。AI 每修一輪就要付出一次完整編譯的成本。
- **`ore check` 要快**：只跑前端、不產生 C，中型模組的目標是 1 秒以內。這是 AI 迭代迴圈的最內圈。
- **`ore fix`**：套用所有機械性的 fix-it，也負責 1.0 之前的語法遷移（把舊語法改寫成新語法），AI 記得的舊寫法不會卡住建置。
- **測試輸出可機讀**：`ore test --json` 與 `--filter`。失敗時輸出 assertion 兩邊的值、panic 位置與 error return trace。doc comment 裡的範例程式碼也會被當成測試執行，文件不會跟實作脫節。
- **`ore fmt` 沒有任何選項**：同一個 AST 只有一種輸出，AI 產生的 diff 不會混入格式雜訊。
- **執行期錯誤可以定位**：panic 一律輸出 `file:line:col: panic: <reason>`，debug 模式另外附上 error return trace。

**代價**：錯誤恢復、fix-it、JSON 診斷都是編譯器前端額外的工程；禁止遮蔽、`pub` 函式必須寫錯誤集，會讓手寫的程式碼稍微囉嗦。換來的是 AI 寫錯時，錯誤在編譯期就以「照著改就好」的形式出現，不會拖到執行期才浮現。

**跟 stage0 現況的差距**（P0 閘門前補上）：stage0 已禁止遮蔽，有 `ore check --json`、錯誤恢復（一次回報所有錯誤）、錯誤代碼（`ore explain`）與 fix-it（`--json` 的 `fixes`、`ore fix`）。

## 並行模型

第一版不在語言層做並行：OS thread、channel、atomic、event loop 都是 std 函式庫，沒有 goroutine，也沒有 `async`。

**為什麼不做 goroutine**：goroutine 需要可成長的 stack 與排程器 runtime，而且會在每個呼叫 C 的地方產生成本（Go 的 cgo 就是例子）。這跟「零成本 C 互通」衝突。

**為什麼暫不做 `async`**：stackless coroutine 會大幅加重編譯器與型別系統的複雜度。Zig 的 async 就曾經整個移除重做。先用 callback + event loop 做出引擎，再根據實際痛點決定要不要加。

**std 提供**

- `std.thread`：`spawn`、`join`、`Pool`（固定大小的 worker pool）。
- `std.sync`：`Mutex`、`RwLock`、`Channel(T)`（有界、MPSC / MPMC）、`Atomic(T)`。
- `std.event`：單執行緒 event loop，底層是 kqueue（macOS）、io\_uring / epoll（Linux）、IOCP（Windows）。

**引擎的執行緒配置（建議）**：main thread 跑 event loop + DOM + JS + style + layout；compositor thread 負責捲動與合成；raster 用 `thread.Pool`；network 跑獨立的 event loop。執行緒之間只用 `Channel` 傳訊息，不共用 DOM。

**Data race 防護**：編譯期不檢查。採用的是前述「只傳訊息」的慣例，加上 CI 跑 ThreadSanitizer（由 C backend 獲得）。

## C 互通與 browser 引擎整合

Ore 與 C 之間沒有邊界成本；與 JS engine 之間的邊界由 `Handle(T)` + 引用計數守住，JS 永遠拿不到 Ore 的裸指標。

**C 互通**

- `extern fn` 宣告 C 函式，`extern struct` 保證 C 配置，`[*]T` 對應 C 指標。
- 第一階段因為輸出的是 C，宣告可以直接對應到 header 的 `#include`，不必重寫型別配置。
- 第二階段加入 `cimport "foo.h"`，用 libclang 讀 header 自動產生宣告。
- Ore 函式加上 `pub extern` 就能以 C ABI 導出，讓既有 C/C++ 程式碼反過來呼叫。

**JS 邊界（無論 JS engine 是自製還是嵌入）**

1. DOM node 活在 Ore 的 `Pool(Node)` 裡。JS wrapper 物件只存 `Handle(Node)`（64 bit）。
2. 每個 slot 有一個 `js_refs` 計數。wrapper 建立時加一，被 JS GC 回收時（finalizer）減一。
3. node 只有在「已從樹上移除」且 `js_refs == 0` 時才真的釋放。
4. 就算邏輯有 bug，wrapper 拿過期的 handle 解參照也只會拿到 `null`，可以轉成 JS 例外，不會造成 use-after-free。

**Binding 產生**：WebIDL → Ore binding 的產生器是工具，不是語言功能。它用 `comptime` 或獨立的 codegen 實作，待引擎需求確定後再設計。

## 編譯器架構與後端

**決策：第一階段只輸出 C，交給 clang 編譯；stage0 編譯器用 Rust 寫。** 前端產生型別化的中間表示 OIR，之後的後端可以替換，不影響語言本身。

&#91;embedded content: 編譯器管線 · 前端 3 段、後端 2 條\]

同一份 OIR 可以走 C + clang（品質與工具），也可以走自有的 native backend（編譯速度）。

**為什麼先做 C backend**

- 最快能跑：不用寫 register allocation、instruction selection、物件檔格式。
- 免費拿到 clang 的 -O2 最佳化、ASan / TSan / UBSan、交叉編譯與 debug info（透過 `#line`）。
- C 互通幾乎零成本：輸出的 C 可以直接 `#include` 對方的 header。
- Nim 與 V 都以 C 為主要後端，Zig 也有 C backend。

**代價與對策**

- C 的 undefined behavior：輸出時加 `-fwrapv -fno-strict-aliasing`，溢位與 bounds 檢查由 Ore 自己生成，不交給 C 語意。
- 編譯速度受 clang 限制：第四階段的 native backend 就是為了 debug build 的速度；release 仍走 C + clang。

**為什麼 stage0 用 Rust**：AST 與 IR 最適合用 enum + `match` 表達，Rust 有而且穩定。Zig 尚未 1.0，每版都有 breaking change；Go 沒有 sum type。std 成熟後再用 Ore 重寫自己（自舉）。

## Roadmap 與待決問題

五個階段依序推進，每段的閘門通過才進下一段；尚未排定日期。

&#91;embedded content: Roadmap · 5 個階段，每段一個閘門\]

P3 是關鍵：只有在真實引擎程式碼上驗證過記憶體模型與 JS 邊界，才決定引擎是否全面採用 Ore。

**待決問題（P0 閘門前要決議）**

- [ ] 正式名稱：Ore 是暫定名。
- [ ] 分號：採用 Go 式自動插入，還是明確寫分號？（AI 角度：傾向 Go 式。AI 熟悉 Go 的規則，`else` 換行、多行呼叫少了尾逗號這類陷阱，由 `ore fmt` 與明確的錯誤訊息擋下。）
- [ ] DOM 字串編碼：UTF-8，還是跟 JS 一致的 UTF-16？影響每次跨邊界的轉換成本。
- [ ] 數學型別（矩陣、色彩）是否開放有限度的 operator overloading？目前只有 `simd`。（AI 角度：傾向不開放，`a * b` 的意思應該只看型別類別就能確定。）
- [ ] `where` 約束要不要在泛型定義時就檢查（類似 Rust），還是只在實例化時檢查？（AI 角度：傾向定義時檢查，錯誤才會落在 AI 正在寫的那一行。）
- [ ] `async` / coroutine 是否列入 v2？等 P3 的 event loop 實戰經驗再決定。
