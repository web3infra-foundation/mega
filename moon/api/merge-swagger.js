#!/usr/bin/env node
const fs = require("fs");
const path = require("path");

const fileBase = path.join(process.cwd(), "api/gen");
const outputFile = path.join(fileBase, "merged_swagger.json");

// 要排除的文件
const excludeFiles = new Set([
  "merged_swagger.json",
  "openapi_schema.json",
]);

// 找到所有 JSON 文件，排除指定的文件
const files = fs
  .readdirSync(fileBase)
  .filter((f) => f.endsWith(".json") && !excludeFiles.has(f))
  .map((f) => path.join(fileBase, f));

if (files.length === 0) {
  console.error("没有找到 JSON 文件可供合并");
  process.exit(1);
}

// 深度合并函数
function deepMerge(target, source) {
  for (const key of Object.keys(source)) {
    if (
      typeof target[key] === "object" &&
      target[key] !== null &&
      !Array.isArray(target[key]) &&
      typeof source[key] === "object" &&
      source[key] !== null &&
      !Array.isArray(source[key])
    ) {
      deepMerge(target[key], source[key]);
    } else {
      target[key] = source[key];
    }
  }
  return target;
}

// 依次读取并 merge
const merged = files
  .map((file) => JSON.parse(fs.readFileSync(file, "utf-8")))
  .reduce(
    (acc, swagger) => {
      acc.info = acc.info && Object.keys(acc.info).length > 0 ? acc.info : swagger.info || {};
      acc.paths = { ...acc.paths, ...(swagger.paths || {}) };
      acc.components = deepMerge(acc.components, swagger.components || {});
      return acc;
    },
    { openapi: "3.0.0", info: {}, paths: {}, components: {} }
  );

// 输出文件
fs.writeFileSync(outputFile, JSON.stringify(merged, null, 2));
console.log(`Swagger JSON 文件合并完成，已生成 ${outputFile} 🎉`);