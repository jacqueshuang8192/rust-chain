fn main() {
    // 新增迁移也必须重编译嵌入式迁移器，避免增量构建继续使用旧迁移列表。
    println!("cargo:rerun-if-changed=migrations");
}
