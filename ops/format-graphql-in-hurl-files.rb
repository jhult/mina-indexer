#!/usr/bin/env ruby
require 'json'
require 'tempfile'
require 'find'

class GraphQLFormatter
  class << self
    def process_directory(dir_path)
      Find.find(dir_path) do |path|
        next unless path.end_with?('.hurl')
        process_file(path)
      end
    end

    def process_file(file_path)
      content = File.read(file_path)
      processed_content = process_graphql_content(content)
      File.write(file_path, processed_content)
    end

    def process_graphql_content(content)
      content.gsub(/```graphql\n(.*?)\n```/m) do |match|
        query_section = $1.strip

        if query_section.include?("variables {")
          query_part, variables_part = query_section.split(/\n\nvariables\s*{/)
          variables_json = variables_part.strip.sub(/}\s*$/, '')
          variables = JSON.parse("{#{variables_json}}")

          query_body = extract_query_body(query_part)
          inlined_query = inline_variables(query_body, variables)
          formatted_query = format_with_biome(inlined_query)
          reordered_query = reorder_arguments(formatted_query)
          "```graphql\n#{reordered_query}\n```"
        else
          formatted_query = format_with_biome(query_section)
          reordered_query = reorder_arguments(formatted_query)
          "```graphql\n#{reordered_query}\n```"
        end
      end
    end

    private

    def extract_query_body(query_part)
      if query_part.include?("query ")
        match = query_part.match(/{\s*(.*)\s*}\s*$/m)
        if match
          content = match[1].strip
          content.split("\n").map(&:strip).join("\n")
        else
          query_part
        end
      else
        query_part
      end
    end

    def inline_variables(query_body, variables)
      variables.each do |key, value|
        query_body.gsub!("$#{key}", format_value(value, key))
      end
      "{\n#{query_body}\n}"
    end

    def format_value(value, key = nil)
      case value
      when Hash
        format_graphql_object(value)
      when String
        if key == "sort_by" || value.match?(/^[A-Z_]+$/)
          value
        else
          "\"#{value}\""
        end
      else
        value.to_s
      end
    end

    def format_graphql_object(obj)
      pairs = obj.map do |k, v|
        formatted_value = format_value(v, k)
        "#{k}: #{formatted_value}"
      end
      "{#{pairs.join(', ')}}"
    end

    def format_with_biome(query)
      Tempfile.create(['query', '.graphql']) do |f|
        f.write(query)
        f.flush
        `biome format --indent-style space --indent-width 2 --write #{f.path}`
        File.read(f.path).strip
      end
    rescue StandardError => e
      puts "Warning: Biome formatting failed (#{e.message}). Using unformatted query."
      query
    end

    def reorder_arguments(query)
      query.gsub(/(\w+)\((.*?)\)/) do |match|
        field_name = $1
        args_str = $2

        args = args_str.split(',').map(&:strip)
        ordered_args = {}

        args.each do |arg|
          key, value = arg.split(':', 2).map(&:strip)
          # Remove quotes from sortBy values
          if key == 'sortBy' && value.match?(/^"[A-Z_]+"$/)
            value = value.gsub('"', '')
          end
          ordered_args[key] = value
        end

        ordered_keys = ['limit', 'sortBy', 'query']
        other_keys = ordered_args.keys - ordered_keys
        final_keys = (ordered_keys & ordered_args.keys) + other_keys

        formatted_args = final_keys.map do |key|
          "#{key}: #{ordered_args[key]}"
        end.join(', ')

        "#{field_name}(#{formatted_args})"
      end
    end
  end
end

if __FILE__ == $PROGRAM_NAME
  if ARGV.empty?
    puts "Usage: #{$0} <directory_or_file>"
    exit 1
  end

  path = ARGV[0]
  if File.directory?(path)
    GraphQLFormatter.process_directory(path)
  else
    GraphQLFormatter.process_file(path)
  end
end
